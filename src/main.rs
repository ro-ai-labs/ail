use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::{Command, ExitCode};

use ail::ail::{
    AilBytecodeProgram, DEFAULT_BASE_LLM_ENDPOINT, ail_document_from_core,
    apply_ail_core_patch_text, apply_ail_flow_edit_text, apply_ail_patch, check_ail_core,
    check_ail_requirements, compile_ail_bytecode_native_elf, compile_ail_core_bytecode,
    compile_ail_core_native_elf, draft_ail_interview, draft_ail_requirements, draft_ail_spec,
    draft_ail_spec_from_requirements, elaborate_ail_core, load_ail_package_dir, parse_ail_bytecode,
    parse_ail_core_text, parse_ail_package_document, parse_ail_package_spec_text,
    parse_ail_patch_text, render_ail_bytecode, render_ail_core, render_ail_flow_view,
    render_ail_runtime_state_lines, render_ail_spec, render_ail_spec_from_core,
    repair_ail_requirements_from_diagnostics, repair_ail_spec_from_diagnostics,
    run_ail_bytecode_action, run_ail_compiler_pass_on_core, run_ail_conformance,
    verify_ail_bytecode,
};
use ail::core_model::json_string;

struct CliOptions {
    runtime_state: BTreeMap<String, String>,
    llm_endpoint: Option<String>,
    artifact_dir: Option<String>,
    patch_path: Option<String>,
    ail_action: Option<String>,
    ail_prompt: Option<String>,
    ail_pass_target: Option<String>,
    ail_build_pass: Option<String>,
    ail_build_agent: Option<String>,
    ail_build_base_model: Option<String>,
    ail_build_target_model: Option<String>,
    ail_interview_file: Option<String>,
    ail_requirements_file: Option<String>,
    ail_spec_file: Option<String>,
    ail_core_file: Option<String>,
    ail_compile_target: Option<String>,
    ail_compile_out: Option<String>,
    ail_compile_all_actions: bool,
    diagnostics_json: bool,
}

struct AilInterviewArtifactSet<'a> {
    package_name: &'a str,
    package_version: &'a str,
    interview_text: &'a str,
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
    if args.len() < 2 || (args[0].as_str() == "ail-patch" && args.len() < 3) {
        return Err(usage());
    }
    let command = &args[0];
    let pathless_core_file_command = args[1].starts_with("--")
        && matches!(
            command.as_str(),
            "ail-spec"
                | "ail-lower"
                | "ail-compile"
                | "ail-build"
                | "ail-run"
                | "ail-patch"
                | "ail-flow-edit"
        );
    let default_path = ".".to_string();
    let (path, option_args): (&String, &[String]) = if pathless_core_file_command {
        (&default_path, &args[1..])
    } else {
        (&args[1], &args[2..])
    };
    let cli_options = parse_cli_options(command, option_args)?;
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
            | "ail-interview"
            | "ail-requirements"
            | "ail-spec"
            | "ail-draft"
            | "ail-build"
            | "ail-pass"
            | "ail-bootstrap"
            | "ail-patch"
            | "ail-flow-edit"
    ) {
        return run_ail_command(command, path, &cli_options);
    }
    Err(format!("unknown AIL command '{command}'"))
}

fn usage() -> String {
    "usage: ail <ail-check|ail-core|ail-flow|ail-flow-edit|ail-lower|ail-compile|ail-run|ail-vm|ail-conformance|ail-interview|ail-requirements|ail-spec|ail-draft|ail-build|ail-pass|ail-bootstrap|ail-patch> <path> [patch|target-package] [--action name] [--prompt text] [--interview-file path] [--requirements-file path] [--spec-file path] [--core-file path] [--pass path] [--agent path] [--target target] [--base-model name] [--target-model name] [--out path] [--all-actions] [--diagnostics-json] [--artifact-dir path] [--llm-endpoint url] [key=value ...]\nsaved-core usage: ail <ail-spec|ail-lower|ail-compile|ail-run|ail-build> --core-file <checked-core> [--action name] [--target target] [--out path] [--artifact-dir path] [key=value ...]\nwasm-contract usage: ail ail-compile <package-or-artifact.ailbc.json> (--action <ActionName>|--all-actions) [--agent <agent-package-or-bytecode>] --target wasm32-unknown-sandbox-wasm --artifact-dir <dir> OR ail ail-compile --core-file <checked-core> (--action <ActionName>|--all-actions) [--agent <agent-package-or-bytecode>] --target wasm32-unknown-sandbox-wasm --artifact-dir <dir>\ncore-patch usage: ail ail-patch --core-file <checked-core> <ail-core.patch.json>\nflow-edit usage: ail ail-flow-edit --core-file <checked-core> <ail-flow.edit.json>\nail-pass usage: ail ail-pass <compiler-pass-package-or-bytecode> <target-package> --action <PassName> [--agent <agent-package-or-bytecode>] [--target linux-x86_64-elf --artifact-dir <dir>] OR ail ail-pass <compiler-pass-package-or-bytecode> --core-file <checked-core> --action <PassName> [--agent <agent-package-or-bytecode>] [--target linux-x86_64-elf --artifact-dir <dir>]\nail-bootstrap usage: ail ail-bootstrap <toolchain-agent-package> --pass <compiler-pass-package> --agent <toolchain-agent-package> --target linux-x86_64-elf --artifact-dir <dir>"
        .to_string()
}

fn render_ail_interview_manifest(artifacts: &AilInterviewArtifactSet<'_>) -> String {
    format!(
        concat!(
            "AIL-Interview-Manifest:\n",
            "package {} {}\n",
            "interview interview.ail-interview.md {}\n"
        ),
        artifacts.package_name,
        artifacts.package_version,
        ail_artifact_fingerprint(artifacts.interview_text)
    )
}

fn write_ail_interview_artifacts(
    artifact_dir: &str,
    artifacts: AilInterviewArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-interview artifact dir {artifact_dir}: {error}")
    })?;
    fs::write(
        root.join("interview.ail-interview.md"),
        artifacts.interview_text,
    )
    .map_err(|error| format!("failed to write ail-interview artifact: {error}"))?;
    fs::write(
        root.join("interview.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(artifacts.interview_text)),
    )
    .map_err(|error| format!("failed to write ail-interview fingerprint artifact: {error}"))?;
    let manifest_text = render_ail_interview_manifest(&artifacts);
    fs::write(root.join("manifest.ail-interview.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-interview manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-interview manifest fingerprint artifact: {error}")
    })?;
    Ok(())
}

fn json_optional_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

fn render_ail_draft_diagnostics_json(
    candidate_artifact: &str,
    diagnostics: &[ail::ail::AilDiagnostic],
) -> String {
    let diagnostics_json = diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                concat!(
                    "    {{",
                    "\"code\":{},",
                    "\"message\":{},",
                    "\"severity\":{},",
                    "\"source_provenance\":{},",
                    "\"affected_graph_item\":{},",
                    "\"repair_suggestion\":{}",
                    "}}"
                ),
                json_string(&diagnostic.code),
                json_string(&diagnostic.message),
                json_string(&diagnostic.severity),
                json_optional_string(diagnostic.source_provenance.as_deref()),
                json_optional_string(diagnostic.affected_graph_item.as_deref()),
                json_optional_string(diagnostic.repair_suggestion.as_deref())
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        concat!(
            "{{\n",
            "  \"candidate_artifact\": {},\n",
            "  \"diagnostics\": [\n",
            "{}\n",
            "  ]\n",
            "}}\n"
        ),
        json_string(candidate_artifact),
        diagnostics_json
    )
}

struct AilBuildArtifactSet<'a> {
    source_manifest_text: Option<&'a str>,
    source_spec_text: Option<&'a str>,
    requirements: Option<&'a str>,
    spec_text: Option<&'a str>,
    core_text: &'a str,
    flow_review_text: &'a str,
    bytecode_text: &'a str,
    bytecode_fingerprint: &'a str,
    prompt_portability_report: Option<&'a str>,
    target_name: Option<&'a str>,
    target_executable: Option<&'a [u8]>,
    native_bytecode_report_text: Option<&'a str>,
    dependency_report_text: Option<&'a str>,
    pass_bytecode_text: Option<&'a str>,
    pass_bytecode_fingerprint: Option<&'a str>,
    pass_trace: Option<&'a [String]>,
    pass_native_executables: &'a [AilNativeArtifact],
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
    agent_native_executables: &'a [AilNativeArtifact],
}

struct AilCompileArtifactSet<'a> {
    source_manifest_text: Option<&'a str>,
    source_spec_text: Option<&'a str>,
    core_text: Option<&'a str>,
    bytecode_text: &'a str,
    action_name: &'a str,
    target_name: &'a str,
    target_executable: &'a [u8],
    native_bytecode_report_text: &'a str,
    dependency_report_text: &'a str,
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
    agent_native_executables: &'a [AilNativeArtifact],
}

struct AilCompileBundleArtifactSet<'a> {
    source_manifest_text: Option<&'a str>,
    source_spec_text: Option<&'a str>,
    core_text: Option<&'a str>,
    bytecode_text: &'a str,
    target_name: &'a str,
    target_executables: &'a [AilNativeArtifact],
    native_bytecode_report_text: &'a str,
    dependency_report_text: &'a str,
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
    agent_native_executables: &'a [AilNativeArtifact],
}

struct AilCompileWasmContractArtifactSet<'a> {
    source_manifest_text: Option<&'a str>,
    source_spec_text: Option<&'a str>,
    core_text: Option<&'a str>,
    bytecode_text: &'a str,
    scope: AilCompileWasmContractScope<'a>,
    target_name: &'a str,
    wasm_contract_report_text: &'a str,
    dependency_report_text: &'a str,
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
}

enum AilCompileWasmContractScope<'a> {
    Action(&'a str),
    AllActions,
}

struct AilBootstrapArtifactSet<'a> {
    target_name: &'a str,
    toolchain_source_manifest_text: &'a str,
    toolchain_source_spec_text: &'a str,
    toolchain_core_text: &'a str,
    toolchain_bytecode_text: &'a str,
    toolchain_conformance_report: &'a str,
    toolchain_native_executables: &'a [AilNativeArtifact],
    compiler_pass_source_manifest_text: &'a str,
    compiler_pass_source_spec_text: &'a str,
    compiler_pass_core_text: &'a str,
    compiler_pass_bytecode_text: &'a str,
    toolchain_pass_output_core_text: &'a str,
    toolchain_pass_trace_text: &'a str,
    fixed_point_report_text: &'a str,
    native_bytecode_report_text: &'a str,
    host_boundary_report_text: &'a str,
    dependency_report_text: &'a str,
    handoff_report_text: &'a str,
    compiler_pass_conformance_report: &'a str,
    compiler_pass_native_executables: &'a [AilNativeArtifact],
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
    compiler_pass_source_manifest_text: Option<&'a str>,
    compiler_pass_source_spec_text: Option<&'a str>,
    target_source_manifest_text: Option<&'a str>,
    target_source_spec_text: Option<&'a str>,
    pass_bytecode_text: &'a str,
    input_core_text: &'a str,
    output_core_text: &'a str,
    trace: &'a [String],
    native_bytecode_report_text: Option<&'a str>,
    dependency_report_text: Option<&'a str>,
    pass_native_executables: &'a [AilNativeArtifact],
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
    agent_native_executables: &'a [AilNativeArtifact],
}

struct AilLowerArtifactSet<'a> {
    source_manifest_text: Option<&'a str>,
    source_spec_text: Option<&'a str>,
    core_text: &'a str,
    bytecode_text: &'a str,
    native_bytecode_report_text: Option<&'a str>,
    dependency_report_text: Option<&'a str>,
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
    agent_native_executables: &'a [AilNativeArtifact],
}

struct AilConformanceArtifactSet<'a> {
    report_text: &'a str,
    native_bytecode_report_text: Option<&'a str>,
    dependency_report_text: Option<&'a str>,
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
    agent_native_executables: &'a [AilNativeArtifact],
}

struct AilConformanceAgentManifestRequest<'a> {
    agent_bytecode: ail::ail::AilBytecodeProgram,
    agent_bytecode_text: String,
    package_name: &'a str,
    report_text: &'a str,
    manifest_text: &'a str,
    manifest_fingerprint: &'a str,
    native_bytecode_report_text: Option<&'a str>,
    dependency_report_text: Option<&'a str>,
}

struct AilLowerAgentManifestRun {
    agent_run: AilBuildAgentRun,
    agent_native_artifacts: Vec<AilNativeArtifact>,
    native_bytecode_report_text: Option<String>,
    dependency_report_text: Option<String>,
}

struct AilSourcePackageArtifacts {
    manifest_text: String,
    spec_text: String,
}

struct AilBuildAgentStart {
    state: BTreeMap<String, String>,
    trace: Vec<String>,
}

struct AilBuildAgentRun {
    bytecode: ail::ail::AilBytecodeProgram,
    bytecode_text: String,
    state: BTreeMap<String, String>,
    trace: Vec<String>,
}

struct AilBuildPromptPortability<'a> {
    base_model: Option<&'a str>,
    target_model: Option<&'a str>,
}

struct AilBuildPassAcceptance<'a> {
    requirements_artifact: Option<&'a str>,
    spec_text: Option<&'a str>,
    core_text: &'a str,
    pass_bytecode_text: &'a str,
    pass_bytecode_fingerprint: &'a str,
    pass_trace: &'a [String],
}

struct AilBuildAgentManifestVerification<'a> {
    manifest_text: &'a str,
    manifest_fingerprint: &'a str,
    source_package_text: Option<&'a str>,
    source_package_fingerprint: Option<&'a str>,
    requirements_fingerprint: Option<&'a str>,
    spec_fingerprint: Option<&'a str>,
    core_fingerprint: &'a str,
    flow_review_fingerprint: &'a str,
    compiler_pass_target_fingerprint: Option<&'a str>,
    prompt_portability_fingerprint: Option<&'a str>,
    native_bytecode_report_text: Option<&'a str>,
    dependency_report_text: Option<&'a str>,
}

fn render_ail_prompt_portability_report(
    base_model: &str,
    target_model: &str,
    requirements_artifact: Option<&str>,
    agent_run: &AilBuildAgentRun,
) -> String {
    let status = agent_run
        .state
        .get("buildrequest.prompt portability report")
        .map(String::as_str)
        .unwrap_or("NotCompared");
    let mut lines = vec![
        "AIL-Prompt-Portability-Report:".to_string(),
        format!("base-model {base_model}"),
        format!("target-model {target_model}"),
        "agent-action CompareAgentPromptPortability".to_string(),
        format!("status {status}"),
    ];
    if let Some(requirements_artifact) = requirements_artifact {
        lines.push(format!(
            "requirements-fingerprint {}",
            ail_artifact_fingerprint(requirements_artifact)
        ));
    }
    lines.push("trace AgentPromptPortabilityCompared".to_string());
    format!("{}\n", lines.join("\n"))
}

fn render_ail_build_manifest(artifacts: &AilBuildArtifactSet<'_>) -> String {
    let mut lines = vec!["AIL-Build-Manifest:".to_string()];
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        lines.push(format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                source_manifest_text,
                source_spec_text,
            ))
        ));
    }
    if let Some(requirements) = artifacts.requirements {
        lines.push(format!(
            "requirements requirements.ail-requirements.md {}",
            ail_artifact_fingerprint(requirements)
        ));
    }
    if let Some(spec_text) = artifacts.spec_text {
        lines.push(format!(
            "spec accepted.ail-spec.md {}",
            ail_artifact_fingerprint(spec_text)
        ));
    }
    lines.push(format!(
        "core checked.ail-core.txt {}",
        ail_artifact_fingerprint(artifacts.core_text)
    ));
    lines.push(format!(
        "flow-review review.ail-flow.json {}",
        ail_artifact_fingerprint(artifacts.flow_review_text)
    ));
    lines.push(format!(
        "bytecode artifact.ailbc.json {}",
        artifacts.bytecode_fingerprint
    ));
    if let Some(prompt_portability_report) = artifacts.prompt_portability_report {
        lines.push(format!(
            "prompt-portability prompt-portability.txt {}",
            ail_artifact_fingerprint(prompt_portability_report)
        ));
    }
    if let (Some(target_name), Some(target_executable)) =
        (artifacts.target_name, artifacts.target_executable)
    {
        lines.push(format!(
            "target {target_name} target.elf {}",
            ail_artifact_fingerprint_bytes(target_executable)
        ));
        lines.push(native_machine_bytecode_manifest_contract_line(target_name));
    }
    if let Some(native_bytecode_report_text) = artifacts.native_bytecode_report_text {
        lines.push(format!(
            "native-bytecode native-bytecode-report.txt {}",
            ail_artifact_fingerprint(native_bytecode_report_text)
        ));
    }
    if let Some(dependency_report_text) = artifacts.dependency_report_text {
        lines.push(format!(
            "dependencies dependency-report.txt {}",
            ail_artifact_fingerprint(dependency_report_text)
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
    for native_pass in artifacts.pass_native_executables {
        lines.push(format!(
            "compiler-pass-target {} {} {}",
            native_pass.target_name,
            native_pass.file_name,
            ail_artifact_fingerprint_bytes(&native_pass.bytes)
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

fn render_ail_compile_manifest(artifacts: &AilCompileArtifactSet<'_>) -> String {
    let mut lines = vec!["AIL-Compile-Manifest:".to_string()];
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        lines.push(format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                source_manifest_text,
                source_spec_text,
            ))
        ));
    }
    if let Some(core_text) = artifacts.core_text {
        lines.push(format!(
            "core checked.ail-core.txt {}",
            ail_artifact_fingerprint(core_text)
        ));
    }
    lines.push(format!(
        "bytecode artifact.ailbc.json {}",
        ail_artifact_fingerprint(artifacts.bytecode_text)
    ));
    lines.push(format!("action {}", artifacts.action_name));
    lines.push(format!(
        "target {} target.elf {}",
        artifacts.target_name,
        ail_artifact_fingerprint_bytes(artifacts.target_executable)
    ));
    lines.push(native_machine_bytecode_manifest_contract_line(
        artifacts.target_name,
    ));
    lines.push(format!(
        "native-bytecode native-bytecode-report.txt {}",
        ail_artifact_fingerprint(artifacts.native_bytecode_report_text)
    ));
    lines.push(format!(
        "dependencies dependency-report.txt {}",
        ail_artifact_fingerprint(artifacts.dependency_report_text)
    ));
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

fn wasm_contract_machine_bytecode_manifest_contract_line(target_name: &str) -> String {
    format!(
        "machine-bytecode-contract {target_name} bytecode-level portable-vm-contract bytecode-container wasm-sandbox-contract bytecode-format wasm32-contract-report"
    )
}

fn render_ail_compile_wasm_contract_manifest(
    artifacts: &AilCompileWasmContractArtifactSet<'_>,
) -> String {
    let mut lines = vec!["AIL-Compile-Manifest:".to_string()];
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        lines.push(format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                source_manifest_text,
                source_spec_text,
            ))
        ));
    }
    if let Some(core_text) = artifacts.core_text {
        lines.push(format!(
            "core checked.ail-core.txt {}",
            ail_artifact_fingerprint(core_text)
        ));
    }
    lines.push(format!(
        "bytecode artifact.ailbc.json {}",
        ail_artifact_fingerprint(artifacts.bytecode_text)
    ));
    match artifacts.scope {
        AilCompileWasmContractScope::Action(action_name) => {
            lines.push(format!("action {action_name}"));
        }
        AilCompileWasmContractScope::AllActions => {
            lines.push("bundle all-actions".to_string());
        }
    }
    lines.push(wasm_contract_machine_bytecode_manifest_contract_line(
        artifacts.target_name,
    ));
    lines.push(format!(
        "wasm-contract wasm-contract-report.txt {}",
        ail_artifact_fingerprint(artifacts.wasm_contract_report_text)
    ));
    lines.push(format!(
        "dependencies dependency-report.txt {}",
        ail_artifact_fingerprint(artifacts.dependency_report_text)
    ));
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        lines.push(format!(
            "agent agent.ailbc.json {}",
            ail_artifact_fingerprint(agent_bytecode_text)
        ));
    }
    if artifacts.agent_trace.is_some() {
        lines.push("trace agent-trace.txt".to_string());
    }
    format!("{}\n", lines.join("\n"))
}

fn render_ail_compile_bundle_manifest(artifacts: &AilCompileBundleArtifactSet<'_>) -> String {
    let mut lines = vec!["AIL-Compile-Manifest:".to_string()];
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        lines.push(format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                source_manifest_text,
                source_spec_text,
            ))
        ));
    }
    if let Some(core_text) = artifacts.core_text {
        lines.push(format!(
            "core checked.ail-core.txt {}",
            ail_artifact_fingerprint(core_text)
        ));
    }
    lines.push(format!(
        "bytecode artifact.ailbc.json {}",
        ail_artifact_fingerprint(artifacts.bytecode_text)
    ));
    lines.push("bundle all-actions".to_string());
    lines.push(native_machine_bytecode_manifest_contract_line(
        artifacts.target_name,
    ));
    for executable in artifacts.target_executables {
        lines.push(format!(
            "target {} {} {}",
            artifacts.target_name,
            executable.file_name,
            ail_artifact_fingerprint_bytes(&executable.bytes)
        ));
    }
    lines.push(format!(
        "native-bytecode native-bytecode-report.txt {}",
        ail_artifact_fingerprint(artifacts.native_bytecode_report_text)
    ));
    lines.push(format!(
        "dependencies dependency-report.txt {}",
        ail_artifact_fingerprint(artifacts.dependency_report_text)
    ));
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

fn ail_bootstrap_source_bundle_text(package_manifest_text: &str, spec_text: &str) -> String {
    format!("ail-package.md:\n{package_manifest_text}\nspec.ail-spec.md:\n{spec_text}")
}

fn load_ail_source_package_artifacts(
    path: &str,
    context: &str,
) -> Result<AilSourcePackageArtifacts, String> {
    load_ail_source_package_artifacts_with_spec_override(path, context, None)
}

fn load_ail_source_package_artifacts_with_spec_override(
    path: &str,
    context: &str,
    spec_override_path: Option<&str>,
) -> Result<AilSourcePackageArtifacts, String> {
    if std::path::Path::new(path).is_file() {
        return Err(format!(
            "{context} requires an AIL package directory so source package evidence can be recorded, found bytecode artifact {path}"
        ));
    }
    let package = load_ail_package_dir(path)?;
    let manifest_path = package.root.join("ail-package.md");
    let manifest_text = fs::read_to_string(&manifest_path).map_err(|error| {
        format!(
            "{context} failed to read source package manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    let spec_text = if let Some(spec_override_path) = spec_override_path {
        fs::read_to_string(spec_override_path).map_err(|error| {
            format!("{context} failed to read source spec override {spec_override_path}: {error}")
        })?
    } else {
        package.spec_text
    };
    Ok(AilSourcePackageArtifacts {
        manifest_text: ensure_trailing_newline(manifest_text),
        spec_text: ensure_trailing_newline(spec_text),
    })
}

fn load_optional_ail_source_package_artifacts(
    path: &str,
    context: &str,
) -> Result<Option<AilSourcePackageArtifacts>, String> {
    if std::path::Path::new(path).is_file() {
        Ok(None)
    } else {
        load_ail_source_package_artifacts(path, context).map(Some)
    }
}

fn write_ail_source_package_snapshot(
    root: &std::path::Path,
    context: &str,
    source_manifest_text: &str,
    source_spec_text: &str,
) -> Result<(), String> {
    write_ail_named_source_package_snapshot(
        root,
        context,
        "source.ail-package.md",
        "source.ail-spec.md",
        "source.fingerprint.txt",
        source_manifest_text,
        source_spec_text,
    )
}

fn write_ail_named_source_package_snapshot(
    root: &std::path::Path,
    context: &str,
    manifest_file_name: &str,
    spec_file_name: &str,
    fingerprint_file_name: &str,
    source_manifest_text: &str,
    source_spec_text: &str,
) -> Result<(), String> {
    fs::write(root.join(manifest_file_name), source_manifest_text)
        .map_err(|error| format!("failed to write {context} source manifest: {error}"))?;
    fs::write(root.join(spec_file_name), source_spec_text)
        .map_err(|error| format!("failed to write {context} source spec: {error}"))?;
    fs::write(
        root.join(fingerprint_file_name),
        format!(
            "{}\n",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                source_manifest_text,
                source_spec_text,
            ))
        ),
    )
    .map_err(|error| format!("failed to write {context} source package fingerprint: {error}"))?;
    Ok(())
}

fn render_ail_bootstrap_manifest(artifacts: &AilBootstrapArtifactSet<'_>) -> String {
    let toolchain_source_bundle = ail_bootstrap_source_bundle_text(
        artifacts.toolchain_source_manifest_text,
        artifacts.toolchain_source_spec_text,
    );
    let compiler_pass_source_bundle = ail_bootstrap_source_bundle_text(
        artifacts.compiler_pass_source_manifest_text,
        artifacts.compiler_pass_source_spec_text,
    );
    let mut lines = vec![
        "AIL-Bootstrap-Manifest:".to_string(),
        format!("target {}", artifacts.target_name),
        native_machine_bytecode_manifest_contract_line(artifacts.target_name),
        "no-host-backend-source true".to_string(),
        format!(
            "toolchain-agent-source toolchain-agent.source.ail-package.md toolchain-agent.source.ail-spec.md {}",
            ail_artifact_fingerprint(&toolchain_source_bundle)
        ),
        format!(
            "toolchain-agent toolchain-agent.ailbc.json {}",
            ail_artifact_fingerprint(artifacts.toolchain_bytecode_text)
        ),
        format!(
            "toolchain-agent-core toolchain-agent.checked.ail-core.txt {}",
            ail_artifact_fingerprint(artifacts.toolchain_core_text)
        ),
        format!(
            "compiler-pass compiler-pass.ailbc.json {}",
            ail_artifact_fingerprint(artifacts.compiler_pass_bytecode_text)
        ),
        format!(
            "compiler-pass-source compiler-pass.source.ail-package.md compiler-pass.source.ail-spec.md {}",
            ail_artifact_fingerprint(&compiler_pass_source_bundle)
        ),
        format!(
            "compiler-pass-core compiler-pass.checked.ail-core.txt {}",
            ail_artifact_fingerprint(artifacts.compiler_pass_core_text)
        ),
        format!(
            "toolchain-agent-pass-output toolchain-agent.pass-output.ail-core.txt {}",
            ail_artifact_fingerprint(artifacts.toolchain_pass_output_core_text)
        ),
        format!(
            "toolchain-agent-pass-trace toolchain-agent.pass-trace.txt {}",
            ail_artifact_fingerprint(artifacts.toolchain_pass_trace_text)
        ),
        format!(
            "bootstrap-fixed-point bootstrap-fixed-point-report.txt {}",
            ail_artifact_fingerprint(artifacts.fixed_point_report_text)
        ),
        format!(
            "bootstrap-native-bytecode bootstrap-native-bytecode-report.txt {}",
            ail_artifact_fingerprint(artifacts.native_bytecode_report_text)
        ),
        format!(
            "bootstrap-host-boundary bootstrap-host-boundary-report.txt {}",
            ail_artifact_fingerprint(artifacts.host_boundary_report_text)
        ),
        format!(
            "bootstrap-dependencies bootstrap-dependency-report.txt {}",
            ail_artifact_fingerprint(artifacts.dependency_report_text)
        ),
        format!(
            "bootstrap-handoff bootstrap-handoff-report.txt {}",
            ail_artifact_fingerprint(artifacts.handoff_report_text)
        ),
        format!(
            "toolchain-agent-conformance toolchain-agent-conformance-report.txt {}",
            ail_artifact_fingerprint(artifacts.toolchain_conformance_report)
        ),
        format!(
            "compiler-pass-conformance compiler-pass-conformance-report.txt {}",
            ail_artifact_fingerprint(artifacts.compiler_pass_conformance_report)
        ),
    ];
    for executable in artifacts.toolchain_native_executables {
        lines.push(format!(
            "toolchain-agent-target {} {} {}",
            executable.target_name,
            executable.file_name,
            ail_artifact_fingerprint_bytes(&executable.bytes)
        ));
    }
    for executable in artifacts.compiler_pass_native_executables {
        lines.push(format!(
            "compiler-pass-target {} {} {}",
            executable.target_name,
            executable.file_name,
            ail_artifact_fingerprint_bytes(&executable.bytes)
        ));
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

fn write_ail_compile_artifacts(
    artifact_dir: &str,
    artifacts: AilCompileArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-compile artifact dir {artifact_dir}: {error}")
    })?;
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        write_ail_source_package_snapshot(
            root,
            "ail-compile",
            source_manifest_text,
            source_spec_text,
        )?;
    }
    if let Some(core_text) = artifacts.core_text {
        fs::write(root.join("checked.ail-core.txt"), core_text)
            .map_err(|error| format!("failed to write ail-compile core artifact: {error}"))?;
        fs::write(
            root.join("checked.ail-core.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(core_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-compile core fingerprint artifact: {error}")
        })?;
    }
    fs::write(root.join("artifact.ailbc.json"), artifacts.bytecode_text)
        .map_err(|error| format!("failed to write ail-compile bytecode artifact: {error}"))?;
    fs::write(
        root.join("artifact.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(artifacts.bytecode_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-compile bytecode fingerprint artifact: {error}")
    })?;
    let target_path = root.join("target.elf");
    fs::write(&target_path, artifacts.target_executable)
        .map_err(|error| format!("failed to write ail-compile target artifact: {error}"))?;
    set_native_executable_permissions(&target_path.to_string_lossy())?;
    fs::write(
        root.join("target.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint_bytes(artifacts.target_executable)
        ),
    )
    .map_err(|error| format!("failed to write ail-compile target fingerprint artifact: {error}"))?;
    fs::write(
        root.join("native-bytecode-report.txt"),
        artifacts.native_bytecode_report_text,
    )
    .map_err(|error| format!("failed to write ail-compile native bytecode report: {error}"))?;
    fs::write(
        root.join("native-bytecode-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.native_bytecode_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-compile native bytecode report fingerprint: {error}")
    })?;
    fs::write(
        root.join("dependency-report.txt"),
        artifacts.dependency_report_text,
    )
    .map_err(|error| format!("failed to write ail-compile dependency report: {error}"))?;
    fs::write(
        root.join("dependency-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.dependency_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-compile dependency report fingerprint: {error}")
    })?;
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        fs::write(root.join("agent.ailbc.json"), agent_bytecode_text).map_err(|error| {
            format!("failed to write ail-compile agent bytecode artifact: {error}")
        })?;
        fs::write(
            root.join("agent.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_bytecode_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-compile agent bytecode fingerprint artifact: {error}")
        })?;
    }
    if let Some(agent_trace) = artifacts.agent_trace {
        fs::write(
            root.join("agent-trace.txt"),
            format!("{}\n", agent_trace.join("\n")),
        )
        .map_err(|error| format!("failed to write ail-compile agent trace artifact: {error}"))?;
    }
    for native_agent in artifacts.agent_native_executables {
        let artifact_path = root.join(&native_agent.file_name);
        fs::write(&artifact_path, &native_agent.bytes).map_err(|error| {
            format!(
                "failed to write ail-compile native agent artifact {}: {error}",
                native_agent.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    let manifest_text = render_ail_compile_manifest(&artifacts);
    fs::write(root.join("manifest.ail-compile.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-compile manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-compile manifest fingerprint artifact: {error}")
    })?;
    Ok(())
}

fn write_ail_compile_wasm_contract_artifacts(
    artifact_dir: &str,
    artifacts: AilCompileWasmContractArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-compile artifact dir {artifact_dir}: {error}")
    })?;
    reject_stale_wasm_contract_executable_artifacts(root, artifact_dir)?;
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        write_ail_source_package_snapshot(
            root,
            "ail-compile",
            source_manifest_text,
            source_spec_text,
        )?;
    }
    if let Some(core_text) = artifacts.core_text {
        fs::write(root.join("checked.ail-core.txt"), core_text)
            .map_err(|error| format!("failed to write ail-compile core artifact: {error}"))?;
        fs::write(
            root.join("checked.ail-core.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(core_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-compile core fingerprint artifact: {error}")
        })?;
    }
    fs::write(root.join("artifact.ailbc.json"), artifacts.bytecode_text)
        .map_err(|error| format!("failed to write ail-compile bytecode artifact: {error}"))?;
    fs::write(
        root.join("artifact.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(artifacts.bytecode_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-compile bytecode fingerprint artifact: {error}")
    })?;
    fs::write(
        root.join("wasm-contract-report.txt"),
        artifacts.wasm_contract_report_text,
    )
    .map_err(|error| format!("failed to write ail-compile wasm contract report: {error}"))?;
    fs::write(
        root.join("wasm-contract-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.wasm_contract_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-compile wasm contract report fingerprint: {error}")
    })?;
    fs::write(
        root.join("dependency-report.txt"),
        artifacts.dependency_report_text,
    )
    .map_err(|error| format!("failed to write ail-compile dependency report: {error}"))?;
    fs::write(
        root.join("dependency-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.dependency_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-compile dependency report fingerprint: {error}")
    })?;
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        fs::write(root.join("agent.ailbc.json"), agent_bytecode_text).map_err(|error| {
            format!("failed to write ail-compile agent bytecode artifact: {error}")
        })?;
        fs::write(
            root.join("agent.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_bytecode_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-compile agent bytecode fingerprint artifact: {error}")
        })?;
    }
    if let Some(agent_trace) = artifacts.agent_trace {
        fs::write(
            root.join("agent-trace.txt"),
            format!("{}\n", agent_trace.join("\n")),
        )
        .map_err(|error| format!("failed to write ail-compile agent trace artifact: {error}"))?;
    }
    let manifest_text = render_ail_compile_wasm_contract_manifest(&artifacts);
    fs::write(root.join("manifest.ail-compile.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-compile manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-compile manifest fingerprint artifact: {error}")
    })?;
    Ok(())
}

fn reject_stale_wasm_contract_executable_artifacts(
    root: &std::path::Path,
    artifact_dir: &str,
) -> Result<(), String> {
    for file_name in [
        "target.elf",
        "target.fingerprint.txt",
        "target.wasm",
        "target.wasm.fingerprint.txt",
        "native-bytecode-report.txt",
        "native-bytecode-report.fingerprint.txt",
    ] {
        if root.join(file_name).exists() {
            return Err(format!(
                "ail-compile wasm contract artifact dir {artifact_dir} contains stale executable artifact {file_name}; use a clean artifact dir"
            ));
        }
    }
    for entry in fs::read_dir(root).map_err(|error| {
        format!("failed to read ail-compile wasm contract artifact dir {artifact_dir}: {error}")
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect ail-compile wasm contract artifact dir {artifact_dir}: {error}"
            )
        })?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if (file_name.starts_with("target-") || file_name.starts_with("agent-"))
            && (file_name.ends_with(".elf") || file_name.ends_with(".wasm"))
        {
            return Err(format!(
                "ail-compile wasm contract artifact dir {artifact_dir} contains stale executable artifact {file_name}; use a clean artifact dir"
            ));
        }
    }
    Ok(())
}

fn write_ail_compile_bundle_artifacts(
    artifact_dir: &str,
    artifacts: AilCompileBundleArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-compile artifact dir {artifact_dir}: {error}")
    })?;
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        write_ail_source_package_snapshot(
            root,
            "ail-compile",
            source_manifest_text,
            source_spec_text,
        )?;
    }
    if let Some(core_text) = artifacts.core_text {
        fs::write(root.join("checked.ail-core.txt"), core_text)
            .map_err(|error| format!("failed to write ail-compile core artifact: {error}"))?;
        fs::write(
            root.join("checked.ail-core.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(core_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-compile core fingerprint artifact: {error}")
        })?;
    }
    fs::write(root.join("artifact.ailbc.json"), artifacts.bytecode_text)
        .map_err(|error| format!("failed to write ail-compile bytecode artifact: {error}"))?;
    fs::write(
        root.join("artifact.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(artifacts.bytecode_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-compile bytecode fingerprint artifact: {error}")
    })?;
    for executable in artifacts.target_executables {
        let artifact_path = root.join(&executable.file_name);
        fs::write(&artifact_path, &executable.bytes).map_err(|error| {
            format!(
                "failed to write ail-compile native target artifact {}: {error}",
                executable.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    fs::write(
        root.join("native-bytecode-report.txt"),
        artifacts.native_bytecode_report_text,
    )
    .map_err(|error| {
        format!("failed to write ail-compile bundle native bytecode report: {error}")
    })?;
    fs::write(
        root.join("native-bytecode-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.native_bytecode_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-compile bundle native bytecode report fingerprint: {error}")
    })?;
    fs::write(
        root.join("dependency-report.txt"),
        artifacts.dependency_report_text,
    )
    .map_err(|error| format!("failed to write ail-compile bundle dependency report: {error}"))?;
    fs::write(
        root.join("dependency-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.dependency_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-compile bundle dependency report fingerprint: {error}")
    })?;
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        fs::write(root.join("agent.ailbc.json"), agent_bytecode_text).map_err(|error| {
            format!("failed to write ail-compile agent bytecode artifact: {error}")
        })?;
        fs::write(
            root.join("agent.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_bytecode_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-compile agent bytecode fingerprint artifact: {error}")
        })?;
    }
    if let Some(agent_trace) = artifacts.agent_trace {
        fs::write(
            root.join("agent-trace.txt"),
            format!("{}\n", agent_trace.join("\n")),
        )
        .map_err(|error| format!("failed to write ail-compile agent trace artifact: {error}"))?;
    }
    for native_agent in artifacts.agent_native_executables {
        let artifact_path = root.join(&native_agent.file_name);
        fs::write(&artifact_path, &native_agent.bytes).map_err(|error| {
            format!(
                "failed to write ail-compile native agent artifact {}: {error}",
                native_agent.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    let manifest_text = render_ail_compile_bundle_manifest(&artifacts);
    fs::write(root.join("manifest.ail-compile.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-compile manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-compile manifest fingerprint artifact: {error}")
    })?;
    Ok(())
}

fn write_ail_bootstrap_artifacts(
    artifact_dir: &str,
    artifacts: AilBootstrapArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-bootstrap artifact dir {artifact_dir}: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.source.ail-package.md"),
        artifacts.toolchain_source_manifest_text,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent source manifest: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.source.ail-spec.md"),
        artifacts.toolchain_source_spec_text,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent source spec: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.source.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                artifacts.toolchain_source_manifest_text,
                artifacts.toolchain_source_spec_text
            ))
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent source fingerprint: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.ailbc.json"),
        artifacts.toolchain_bytecode_text,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent bytecode artifact: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.toolchain_bytecode_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent fingerprint artifact: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.checked.ail-core.txt"),
        artifacts.toolchain_core_text,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent checked core artifact: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.core.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.toolchain_core_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent core fingerprint artifact: {error}")
    })?;
    fs::write(
        root.join("compiler-pass.source.ail-package.md"),
        artifacts.compiler_pass_source_manifest_text,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap compiler pass source manifest: {error}")
    })?;
    fs::write(
        root.join("compiler-pass.source.ail-spec.md"),
        artifacts.compiler_pass_source_spec_text,
    )
    .map_err(|error| format!("failed to write ail-bootstrap compiler pass source spec: {error}"))?;
    fs::write(
        root.join("compiler-pass.source.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                artifacts.compiler_pass_source_manifest_text,
                artifacts.compiler_pass_source_spec_text
            ))
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap compiler pass source fingerprint: {error}")
    })?;
    fs::write(
        root.join("compiler-pass.ailbc.json"),
        artifacts.compiler_pass_bytecode_text,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap compiler pass bytecode artifact: {error}")
    })?;
    fs::write(
        root.join("compiler-pass.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.compiler_pass_bytecode_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap compiler pass fingerprint artifact: {error}")
    })?;
    fs::write(
        root.join("compiler-pass.checked.ail-core.txt"),
        artifacts.compiler_pass_core_text,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap compiler pass checked core artifact: {error}")
    })?;
    fs::write(
        root.join("compiler-pass.core.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.compiler_pass_core_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap compiler pass core fingerprint artifact: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.pass-output.ail-core.txt"),
        artifacts.toolchain_pass_output_core_text,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent pass output core artifact: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.pass-output.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.toolchain_pass_output_core_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent pass output fingerprint: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.pass-trace.txt"),
        artifacts.toolchain_pass_trace_text,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent pass trace artifact: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent.pass-trace.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.toolchain_pass_trace_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain agent pass trace fingerprint: {error}")
    })?;
    fs::write(
        root.join("bootstrap-fixed-point-report.txt"),
        artifacts.fixed_point_report_text,
    )
    .map_err(|error| format!("failed to write ail-bootstrap fixed point report: {error}"))?;
    fs::write(
        root.join("bootstrap-fixed-point-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.fixed_point_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap fixed point report fingerprint: {error}")
    })?;
    fs::write(
        root.join("bootstrap-native-bytecode-report.txt"),
        artifacts.native_bytecode_report_text,
    )
    .map_err(|error| format!("failed to write ail-bootstrap native bytecode report: {error}"))?;
    fs::write(
        root.join("bootstrap-native-bytecode-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.native_bytecode_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap native bytecode report fingerprint: {error}")
    })?;
    fs::write(
        root.join("bootstrap-host-boundary-report.txt"),
        artifacts.host_boundary_report_text,
    )
    .map_err(|error| format!("failed to write ail-bootstrap host boundary report: {error}"))?;
    fs::write(
        root.join("bootstrap-host-boundary-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.host_boundary_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap host boundary report fingerprint: {error}")
    })?;
    fs::write(
        root.join("bootstrap-dependency-report.txt"),
        artifacts.dependency_report_text,
    )
    .map_err(|error| format!("failed to write ail-bootstrap dependency report: {error}"))?;
    fs::write(
        root.join("bootstrap-dependency-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.dependency_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap dependency report fingerprint: {error}")
    })?;
    fs::write(
        root.join("bootstrap-handoff-report.txt"),
        artifacts.handoff_report_text,
    )
    .map_err(|error| format!("failed to write ail-bootstrap handoff report: {error}"))?;
    fs::write(
        root.join("bootstrap-handoff-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.handoff_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap handoff report fingerprint: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent-conformance-report.txt"),
        artifacts.toolchain_conformance_report,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain conformance report: {error}")
    })?;
    fs::write(
        root.join("toolchain-agent-conformance-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.toolchain_conformance_report)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap toolchain conformance fingerprint: {error}")
    })?;
    fs::write(
        root.join("compiler-pass-conformance-report.txt"),
        artifacts.compiler_pass_conformance_report,
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap compiler pass conformance report: {error}")
    })?;
    fs::write(
        root.join("compiler-pass-conformance-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.compiler_pass_conformance_report)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap compiler pass conformance fingerprint: {error}")
    })?;
    for executable in artifacts.toolchain_native_executables {
        let artifact_path = root.join(&executable.file_name);
        fs::write(&artifact_path, &executable.bytes).map_err(|error| {
            format!(
                "failed to write ail-bootstrap native toolchain artifact {}: {error}",
                executable.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    for executable in artifacts.compiler_pass_native_executables {
        let artifact_path = root.join(&executable.file_name);
        fs::write(&artifact_path, &executable.bytes).map_err(|error| {
            format!(
                "failed to write ail-bootstrap native compiler pass artifact {}: {error}",
                executable.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        fs::write(root.join("agent.ailbc.json"), agent_bytecode_text).map_err(|error| {
            format!("failed to write ail-bootstrap agent bytecode artifact: {error}")
        })?;
        fs::write(
            root.join("agent.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_bytecode_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-bootstrap agent fingerprint artifact: {error}")
        })?;
    }
    if let Some(agent_trace) = artifacts.agent_trace {
        fs::write(
            root.join("agent-trace.txt"),
            format!("{}\n", agent_trace.join("\n")),
        )
        .map_err(|error| format!("failed to write ail-bootstrap agent trace artifact: {error}"))?;
    }
    for native_agent in artifacts.agent_native_executables {
        let artifact_path = root.join(&native_agent.file_name);
        fs::write(&artifact_path, &native_agent.bytes).map_err(|error| {
            format!(
                "failed to write ail-bootstrap native agent artifact {}: {error}",
                native_agent.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    let manifest_text = render_ail_bootstrap_manifest(&artifacts);
    fs::write(root.join("manifest.ail-bootstrap.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-bootstrap manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap manifest fingerprint artifact: {error}")
    })?;
    Ok(())
}

fn write_ail_build_artifacts(
    artifact_dir: &str,
    artifacts: AilBuildArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-build artifact dir {artifact_dir}: {error}")
    })?;
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        fs::write(root.join("source.ail-package.md"), source_manifest_text)
            .map_err(|error| format!("failed to write ail-build source manifest: {error}"))?;
        fs::write(root.join("source.ail-spec.md"), source_spec_text)
            .map_err(|error| format!("failed to write ail-build source spec: {error}"))?;
        fs::write(
            root.join("source.fingerprint.txt"),
            format!(
                "{}\n",
                ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                    source_manifest_text,
                    source_spec_text,
                ))
            ),
        )
        .map_err(|error| {
            format!("failed to write ail-build source package fingerprint: {error}")
        })?;
    }
    if let Some(requirements) = artifacts.requirements {
        fs::write(root.join("requirements.ail-requirements.md"), requirements)
            .map_err(|error| format!("failed to write ail-build requirements artifact: {error}"))?;
        fs::write(
            root.join("requirements.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(requirements)),
        )
        .map_err(|error| {
            format!("failed to write ail-build requirements fingerprint artifact: {error}")
        })?;
    }
    if let Some(spec_text) = artifacts.spec_text {
        fs::write(root.join("accepted.ail-spec.md"), spec_text)
            .map_err(|error| format!("failed to write ail-build spec artifact: {error}"))?;
        fs::write(
            root.join("accepted.ail-spec.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(spec_text)),
        )
        .map_err(|error| format!("failed to write ail-build spec fingerprint artifact: {error}"))?;
    }
    fs::write(root.join("checked.ail-core.txt"), artifacts.core_text)
        .map_err(|error| format!("failed to write ail-build core artifact: {error}"))?;
    fs::write(
        root.join("checked.ail-core.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(artifacts.core_text)),
    )
    .map_err(|error| format!("failed to write ail-build core fingerprint artifact: {error}"))?;
    fs::write(
        root.join("review.ail-flow.json"),
        artifacts.flow_review_text,
    )
    .map_err(|error| format!("failed to write ail-build flow review artifact: {error}"))?;
    fs::write(
        root.join("review.ail-flow.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(artifacts.flow_review_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-build flow review fingerprint artifact: {error}")
    })?;
    fs::write(root.join("artifact.ailbc.json"), artifacts.bytecode_text)
        .map_err(|error| format!("failed to write ail-build bytecode artifact: {error}"))?;
    fs::write(
        root.join("artifact.fingerprint.txt"),
        format!("{}\n", artifacts.bytecode_fingerprint),
    )
    .map_err(|error| format!("failed to write ail-build bytecode fingerprint artifact: {error}"))?;
    if let Some(prompt_portability_report) = artifacts.prompt_portability_report {
        fs::write(
            root.join("prompt-portability.txt"),
            prompt_portability_report,
        )
        .map_err(|error| {
            format!("failed to write ail-build prompt portability artifact: {error}")
        })?;
        fs::write(
            root.join("prompt-portability.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(prompt_portability_report)),
        )
        .map_err(|error| {
            format!("failed to write ail-build prompt portability fingerprint artifact: {error}")
        })?;
    }
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
    if let Some(native_bytecode_report_text) = artifacts.native_bytecode_report_text {
        fs::write(
            root.join("native-bytecode-report.txt"),
            native_bytecode_report_text,
        )
        .map_err(|error| {
            format!("failed to write ail-build native bytecode report artifact: {error}")
        })?;
        fs::write(
            root.join("native-bytecode-report.fingerprint.txt"),
            format!(
                "{}\n",
                ail_artifact_fingerprint(native_bytecode_report_text)
            ),
        )
        .map_err(|error| {
            format!(
                "failed to write ail-build native bytecode report fingerprint artifact: {error}"
            )
        })?;
    }
    if let Some(dependency_report_text) = artifacts.dependency_report_text {
        fs::write(root.join("dependency-report.txt"), dependency_report_text)
            .map_err(|error| format!("failed to write ail-build dependency report: {error}"))?;
        fs::write(
            root.join("dependency-report.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(dependency_report_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-build dependency report fingerprint: {error}")
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
    for native_pass in artifacts.pass_native_executables {
        let artifact_path = root.join(&native_pass.file_name);
        fs::write(&artifact_path, &native_pass.bytes).map_err(|error| {
            format!(
                "failed to write ail-build native compiler-pass artifact {}: {error}",
                native_pass.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
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

fn render_ail_lower_manifest(artifacts: &AilLowerArtifactSet<'_>) -> String {
    let mut lines = vec!["AIL-Lower-Manifest:".to_string()];
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        lines.push(format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                source_manifest_text,
                source_spec_text,
            ))
        ));
    }
    lines.extend([
        format!(
            "core checked.ail-core.txt {}",
            ail_artifact_fingerprint(artifacts.core_text)
        ),
        format!(
            "bytecode artifact.ailbc.json {}",
            ail_artifact_fingerprint(artifacts.bytecode_text)
        ),
    ]);
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        lines.push(format!(
            "agent agent.ailbc.json {}",
            ail_artifact_fingerprint(agent_bytecode_text)
        ));
    }
    if artifacts.agent_trace.is_some() {
        lines.push("trace agent-trace.txt".to_string());
    }
    if let Some(contract_line) = native_machine_bytecode_manifest_contract_line_from_artifacts(&[
        artifacts.agent_native_executables,
    ]) {
        lines.push(contract_line);
    }
    if let Some(native_bytecode_report_text) = artifacts.native_bytecode_report_text {
        lines.push(format!(
            "native-bytecode native-bytecode-report.txt {}",
            ail_artifact_fingerprint(native_bytecode_report_text)
        ));
    }
    if let Some(dependency_report_text) = artifacts.dependency_report_text {
        lines.push(format!(
            "dependencies dependency-report.txt {}",
            ail_artifact_fingerprint(dependency_report_text)
        ));
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

fn write_ail_lower_artifacts(
    artifact_dir: &str,
    artifacts: AilLowerArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-lower artifact dir {artifact_dir}: {error}")
    })?;
    if let (Some(source_manifest_text), Some(source_spec_text)) =
        (artifacts.source_manifest_text, artifacts.source_spec_text)
    {
        fs::write(root.join("source.ail-package.md"), source_manifest_text)
            .map_err(|error| format!("failed to write ail-lower source manifest: {error}"))?;
        fs::write(root.join("source.ail-spec.md"), source_spec_text)
            .map_err(|error| format!("failed to write ail-lower source spec: {error}"))?;
        fs::write(
            root.join("source.fingerprint.txt"),
            format!(
                "{}\n",
                ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                    source_manifest_text,
                    source_spec_text,
                ))
            ),
        )
        .map_err(|error| {
            format!("failed to write ail-lower source package fingerprint: {error}")
        })?;
    }
    fs::write(root.join("checked.ail-core.txt"), artifacts.core_text)
        .map_err(|error| format!("failed to write ail-lower core artifact: {error}"))?;
    fs::write(
        root.join("checked.ail-core.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(artifacts.core_text)),
    )
    .map_err(|error| format!("failed to write ail-lower core fingerprint artifact: {error}"))?;
    fs::write(root.join("artifact.ailbc.json"), artifacts.bytecode_text)
        .map_err(|error| format!("failed to write ail-lower bytecode artifact: {error}"))?;
    let bytecode_fingerprint = ail_artifact_fingerprint(artifacts.bytecode_text);
    fs::write(
        root.join("artifact.fingerprint.txt"),
        format!("{bytecode_fingerprint}\n"),
    )
    .map_err(|error| format!("failed to write ail-lower bytecode fingerprint artifact: {error}"))?;
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        fs::write(root.join("agent.ailbc.json"), agent_bytecode_text).map_err(|error| {
            format!("failed to write ail-lower agent bytecode artifact: {error}")
        })?;
        fs::write(
            root.join("agent.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_bytecode_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-lower agent bytecode fingerprint artifact: {error}")
        })?;
    }
    if let Some(agent_trace) = artifacts.agent_trace {
        fs::write(
            root.join("agent-trace.txt"),
            format!("{}\n", agent_trace.join("\n")),
        )
        .map_err(|error| format!("failed to write ail-lower agent trace artifact: {error}"))?;
    }
    if let Some(native_bytecode_report_text) = artifacts.native_bytecode_report_text {
        fs::write(
            root.join("native-bytecode-report.txt"),
            native_bytecode_report_text,
        )
        .map_err(|error| format!("failed to write ail-lower native bytecode report: {error}"))?;
        fs::write(
            root.join("native-bytecode-report.fingerprint.txt"),
            format!(
                "{}\n",
                ail_artifact_fingerprint(native_bytecode_report_text)
            ),
        )
        .map_err(|error| {
            format!("failed to write ail-lower native bytecode report fingerprint: {error}")
        })?;
    }
    if let Some(dependency_report_text) = artifacts.dependency_report_text {
        fs::write(root.join("dependency-report.txt"), dependency_report_text)
            .map_err(|error| format!("failed to write ail-lower dependency report: {error}"))?;
        fs::write(
            root.join("dependency-report.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(dependency_report_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-lower dependency report fingerprint: {error}")
        })?;
    }
    for native_agent in artifacts.agent_native_executables {
        let artifact_path = root.join(&native_agent.file_name);
        fs::write(&artifact_path, &native_agent.bytes).map_err(|error| {
            format!(
                "failed to write ail-lower native agent artifact {}: {error}",
                native_agent.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    let manifest_text = render_ail_lower_manifest(&artifacts);
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

fn render_ail_conformance_report(result: &ail::ail::AilConformanceResult) -> String {
    let mut lines = vec![format!("ail conformance: package {}", result.package_name)];
    if result.accepted_diagnostics.is_empty() {
        lines.push(format!("valid: {}", result.accepted_fixture));
    } else {
        for diagnostic in &result.accepted_diagnostics {
            lines.push(format!(
                "valid: {} {}",
                result.accepted_fixture,
                diagnostic.detailed_message()
            ));
        }
    }
    for fixture in &result.accepted {
        if fixture.diagnostics.is_empty() {
            lines.push(format!("accepted: {}", fixture.fixture));
        } else {
            for diagnostic in &fixture.diagnostics {
                lines.push(format!(
                    "accepted: {} {}",
                    fixture.fixture,
                    diagnostic.detailed_message()
                ));
            }
        }
    }
    for fixture in &result.rejected {
        if fixture.diagnostics.is_empty() {
            lines.push(format!(
                "rejected: {} unexpectedly accepted",
                fixture.fixture
            ));
        } else {
            for diagnostic in &fixture.diagnostics {
                lines.push(format!(
                    "rejected: {} {}",
                    fixture.fixture,
                    diagnostic.detailed_message()
                ));
            }
        }
    }
    if result.success() {
        lines.push("ail conformance: ok".to_string());
    } else {
        lines.push("ail conformance: failed".to_string());
    }
    format!("{}\n", lines.join("\n"))
}

fn render_ail_conformance_manifest(
    result: &ail::ail::AilConformanceResult,
    artifacts: &AilConformanceArtifactSet<'_>,
) -> String {
    let mut lines = vec![
        "AIL-Conformance-Manifest:".to_string(),
        format!("package {}", result.package_name),
        format!(
            "report conformance-report.txt {}",
            ail_artifact_fingerprint(artifacts.report_text)
        ),
        format!("valid {}", result.accepted_fixture),
    ];
    for fixture in &result.accepted {
        lines.push(format!("accepted {}", fixture.fixture));
    }
    for fixture in &result.rejected {
        lines.push(format!("rejected {}", fixture.fixture));
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
    if let Some(contract_line) = native_machine_bytecode_manifest_contract_line_from_artifacts(&[
        artifacts.agent_native_executables,
    ]) {
        lines.push(contract_line);
    }
    if let Some(native_bytecode_report_text) = artifacts.native_bytecode_report_text {
        lines.push(format!(
            "native-bytecode native-bytecode-report.txt {}",
            ail_artifact_fingerprint(native_bytecode_report_text)
        ));
    }
    if let Some(dependency_report_text) = artifacts.dependency_report_text {
        lines.push(format!(
            "dependencies dependency-report.txt {}",
            ail_artifact_fingerprint(dependency_report_text)
        ));
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

fn write_ail_conformance_artifacts(
    artifact_dir: &str,
    result: &ail::ail::AilConformanceResult,
    artifacts: AilConformanceArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-conformance artifact dir {artifact_dir}: {error}")
    })?;
    fs::write(root.join("conformance-report.txt"), artifacts.report_text)
        .map_err(|error| format!("failed to write ail-conformance report artifact: {error}"))?;
    fs::write(
        root.join("conformance-report.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(artifacts.report_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-conformance report fingerprint artifact: {error}")
    })?;
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        fs::write(root.join("agent.ailbc.json"), agent_bytecode_text).map_err(|error| {
            format!("failed to write ail-conformance agent bytecode artifact: {error}")
        })?;
        fs::write(
            root.join("agent.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_bytecode_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-conformance agent bytecode fingerprint artifact: {error}")
        })?;
    }
    if let Some(agent_trace) = artifacts.agent_trace {
        fs::write(
            root.join("agent-trace.txt"),
            format!("{}\n", agent_trace.join("\n")),
        )
        .map_err(|error| {
            format!("failed to write ail-conformance agent trace artifact: {error}")
        })?;
    }
    if let Some(native_bytecode_report_text) = artifacts.native_bytecode_report_text {
        fs::write(
            root.join("native-bytecode-report.txt"),
            native_bytecode_report_text,
        )
        .map_err(|error| {
            format!("failed to write ail-conformance native bytecode report: {error}")
        })?;
        fs::write(
            root.join("native-bytecode-report.fingerprint.txt"),
            format!(
                "{}\n",
                ail_artifact_fingerprint(native_bytecode_report_text)
            ),
        )
        .map_err(|error| {
            format!("failed to write ail-conformance native bytecode report fingerprint: {error}")
        })?;
    }
    if let Some(dependency_report_text) = artifacts.dependency_report_text {
        fs::write(root.join("dependency-report.txt"), dependency_report_text).map_err(|error| {
            format!("failed to write ail-conformance dependency report: {error}")
        })?;
        fs::write(
            root.join("dependency-report.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(dependency_report_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-conformance dependency report fingerprint: {error}")
        })?;
    }
    for native_agent in artifacts.agent_native_executables {
        let artifact_path = root.join(&native_agent.file_name);
        fs::write(&artifact_path, &native_agent.bytes).map_err(|error| {
            format!(
                "failed to write ail-conformance native agent artifact {}: {error}",
                native_agent.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    let manifest_text = render_ail_conformance_manifest(result, &artifacts);
    fs::write(root.join("manifest.ail-conformance.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-conformance manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-conformance manifest fingerprint artifact: {error}")
    })?;
    Ok(())
}

fn render_ail_pass_manifest(artifacts: &AilPassArtifactSet<'_>) -> String {
    let pass_fingerprint = ail_artifact_fingerprint(artifacts.pass_bytecode_text);
    let mut lines = vec!["AIL-Pass-Manifest:".to_string()];
    if let (Some(source_manifest_text), Some(source_spec_text)) = (
        artifacts.compiler_pass_source_manifest_text,
        artifacts.compiler_pass_source_spec_text,
    ) {
        lines.push(format!(
            "compiler-pass-source compiler-pass.source.ail-package.md compiler-pass.source.ail-spec.md {}",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                source_manifest_text,
                source_spec_text,
            ))
        ));
    }
    if let (Some(source_manifest_text), Some(source_spec_text)) = (
        artifacts.target_source_manifest_text,
        artifacts.target_source_spec_text,
    ) {
        lines.push(format!(
            "target-source target.source.ail-package.md target.source.ail-spec.md {}",
            ail_artifact_fingerprint(&ail_bootstrap_source_bundle_text(
                source_manifest_text,
                source_spec_text,
            ))
        ));
    }
    lines.push(format!("compiler-pass pass.ailbc.json {pass_fingerprint}"));
    for native_pass in artifacts.pass_native_executables {
        lines.push(format!(
            "compiler-pass-target {} {} {}",
            native_pass.target_name,
            native_pass.file_name,
            ail_artifact_fingerprint_bytes(&native_pass.bytes)
        ));
    }
    if let Some(contract_line) = native_machine_bytecode_manifest_contract_line_from_artifacts(&[
        artifacts.pass_native_executables,
        artifacts.agent_native_executables,
    ]) {
        lines.push(contract_line);
    }
    if let Some(native_bytecode_report_text) = artifacts.native_bytecode_report_text {
        lines.push(format!(
            "native-bytecode native-bytecode-report.txt {}",
            ail_artifact_fingerprint(native_bytecode_report_text)
        ));
    }
    if let Some(dependency_report_text) = artifacts.dependency_report_text {
        lines.push(format!(
            "dependencies dependency-report.txt {}",
            ail_artifact_fingerprint(dependency_report_text)
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
    if let (Some(source_manifest_text), Some(source_spec_text)) = (
        artifacts.compiler_pass_source_manifest_text,
        artifacts.compiler_pass_source_spec_text,
    ) {
        write_ail_named_source_package_snapshot(
            root,
            "ail-pass compiler pass",
            "compiler-pass.source.ail-package.md",
            "compiler-pass.source.ail-spec.md",
            "compiler-pass.source.fingerprint.txt",
            source_manifest_text,
            source_spec_text,
        )?;
    }
    if let (Some(source_manifest_text), Some(source_spec_text)) = (
        artifacts.target_source_manifest_text,
        artifacts.target_source_spec_text,
    ) {
        write_ail_named_source_package_snapshot(
            root,
            "ail-pass target",
            "target.source.ail-package.md",
            "target.source.ail-spec.md",
            "target.source.fingerprint.txt",
            source_manifest_text,
            source_spec_text,
        )?;
    }
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
    if let Some(native_bytecode_report_text) = artifacts.native_bytecode_report_text {
        fs::write(
            root.join("native-bytecode-report.txt"),
            native_bytecode_report_text,
        )
        .map_err(|error| format!("failed to write ail-pass native bytecode report: {error}"))?;
        fs::write(
            root.join("native-bytecode-report.fingerprint.txt"),
            format!(
                "{}\n",
                ail_artifact_fingerprint(native_bytecode_report_text)
            ),
        )
        .map_err(|error| {
            format!("failed to write ail-pass native bytecode report fingerprint: {error}")
        })?;
    }
    if let Some(dependency_report_text) = artifacts.dependency_report_text {
        fs::write(root.join("dependency-report.txt"), dependency_report_text)
            .map_err(|error| format!("failed to write ail-pass dependency report: {error}"))?;
        fs::write(
            root.join("dependency-report.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(dependency_report_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-pass dependency report fingerprint: {error}")
        })?;
    }
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

    let compiler_pass_source_artifacts =
        load_optional_ail_source_package_artifacts(pass_path, "ail-pass compiler pass")?;
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
    let target_source_artifacts = load_ail_pass_target_source_artifacts(cli_options)?;
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
    let native_bytecode_report_text = if let Some(target) = &cli_options.ail_compile_target {
        Some(render_ail_pass_native_bytecode_report(
            target,
            pass_native_artifacts.as_slice(),
            agent_native_artifacts.as_slice(),
        )?)
    } else {
        None
    };
    let dependency_report_text = if let Some(target) = &cli_options.ail_compile_target {
        Some(render_ail_pass_dependency_report(
            target,
            pass_native_artifacts.as_slice(),
            agent_native_artifacts.as_slice(),
        )?)
    } else {
        None
    };
    if let (Some(agent_run), Some(_artifact_dir)) =
        (agent_run.as_mut(), cli_options.artifact_dir.as_ref())
    {
        let manifest_text = render_ail_pass_manifest(&AilPassArtifactSet {
            compiler_pass_source_manifest_text: compiler_pass_source_artifacts
                .as_ref()
                .map(|artifacts| artifacts.manifest_text.as_str()),
            compiler_pass_source_spec_text: compiler_pass_source_artifacts
                .as_ref()
                .map(|artifacts| artifacts.spec_text.as_str()),
            target_source_manifest_text: target_source_artifacts
                .as_ref()
                .map(|artifacts| artifacts.manifest_text.as_str()),
            target_source_spec_text: target_source_artifacts
                .as_ref()
                .map(|artifacts| artifacts.spec_text.as_str()),
            pass_bytecode_text: &pass_bytecode_text,
            input_core_text: &input_core_text,
            output_core_text: &output_core_text,
            trace: &result.run.trace,
            native_bytecode_report_text: native_bytecode_report_text.as_deref(),
            dependency_report_text: dependency_report_text.as_deref(),
            pass_native_executables: pass_native_artifacts.as_slice(),
            agent_bytecode_text: Some(agent_run.bytecode_text.as_str()),
            agent_trace: Some(agent_run.trace.as_slice()),
            agent_native_executables: agent_native_artifacts.as_slice(),
        });
        let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
        run_ail_pass_agent_verify_manifest(
            agent_run,
            &manifest_text,
            &manifest_fingerprint,
            compiler_pass_source_artifacts.as_ref(),
            target_source_artifacts.as_ref(),
            native_bytecode_report_text.as_deref(),
            dependency_report_text.as_deref(),
        )?;
    }
    if let Some(artifact_dir) = &cli_options.artifact_dir {
        write_ail_pass_artifacts(
            artifact_dir,
            AilPassArtifactSet {
                compiler_pass_source_manifest_text: compiler_pass_source_artifacts
                    .as_ref()
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                compiler_pass_source_spec_text: compiler_pass_source_artifacts
                    .as_ref()
                    .map(|artifacts| artifacts.spec_text.as_str()),
                target_source_manifest_text: target_source_artifacts
                    .as_ref()
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                target_source_spec_text: target_source_artifacts
                    .as_ref()
                    .map(|artifacts| artifacts.spec_text.as_str()),
                pass_bytecode_text: &pass_bytecode_text,
                input_core_text: &input_core_text,
                output_core_text: &output_core_text,
                trace: &result.run.trace,
                native_bytecode_report_text: native_bytecode_report_text.as_deref(),
                dependency_report_text: dependency_report_text.as_deref(),
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
    compiler_pass_source_artifacts: Option<&AilSourcePackageArtifacts>,
    target_source_artifacts: Option<&AilSourcePackageArtifacts>,
    native_bytecode_report_text: Option<&str>,
    dependency_report_text: Option<&str>,
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
    if let Some(compiler_pass_source_artifacts) = compiler_pass_source_artifacts {
        let source_package_text = ail_bootstrap_source_bundle_text(
            &compiler_pass_source_artifacts.manifest_text,
            &compiler_pass_source_artifacts.spec_text,
        );
        verify_state.insert(
            "buildrequest.compiler pass source package".to_string(),
            source_package_text.clone(),
        );
        verify_state.insert(
            "buildrequest.compiler pass source package fingerprint".to_string(),
            ail_artifact_fingerprint(&source_package_text),
        );
    }
    if let Some(target_source_artifacts) = target_source_artifacts {
        let source_package_text = ail_bootstrap_source_bundle_text(
            &target_source_artifacts.manifest_text,
            &target_source_artifacts.spec_text,
        );
        verify_state.insert(
            "buildrequest.source package".to_string(),
            source_package_text.clone(),
        );
        verify_state.insert(
            "buildrequest.source package fingerprint".to_string(),
            ail_artifact_fingerprint(&source_package_text),
        );
    }
    verify_state.insert(
        "buildrequest.machine bytecode contract".to_string(),
        machine_bytecode_contract_from_native_report(native_bytecode_report_text),
    );
    if let Some(native_bytecode_report_text) = native_bytecode_report_text {
        verify_state.insert(
            "buildrequest.native bytecode report".to_string(),
            native_bytecode_report_text.to_string(),
        );
        verify_state.insert(
            "buildrequest.native bytecode report fingerprint".to_string(),
            ail_artifact_fingerprint(native_bytecode_report_text),
        );
    }
    if let Some(dependency_report_text) = dependency_report_text {
        verify_state.insert(
            "buildrequest.dependency report".to_string(),
            dependency_report_text.to_string(),
        );
        verify_state.insert(
            "buildrequest.dependency report fingerprint".to_string(),
            ail_artifact_fingerprint(dependency_report_text),
        );
    }
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

fn run_ail_conformance_agent_verify_manifest(
    request: AilConformanceAgentManifestRequest<'_>,
) -> Result<AilBuildAgentRun, String> {
    let AilConformanceAgentManifestRequest {
        agent_bytecode,
        agent_bytecode_text,
        package_name,
        report_text,
        manifest_text,
        manifest_fingerprint,
        native_bytecode_report_text,
        dependency_report_text,
    } = request;
    if !agent_bytecode
        .actions
        .contains_key("VerifyConformanceManifest")
    {
        return Err(
            "ail-conformance --agent requires a VerifyConformanceManifest action".to_string(),
        );
    }
    let mut state = BTreeMap::from([
        (
            "buildrequest.id".to_string(),
            format!("{package_name}-conformance"),
        ),
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
            "buildrequest.conformance report".to_string(),
            report_text.to_string(),
        ),
        (
            "buildrequest.conformance report fingerprint".to_string(),
            ail_artifact_fingerprint(report_text),
        ),
        (
            "buildrequest.artifact manifest".to_string(),
            manifest_text.to_string(),
        ),
        (
            "buildrequest.artifact manifest fingerprint".to_string(),
            manifest_fingerprint.to_string(),
        ),
        (
            "buildrequest.status".to_string(),
            "BytecodeReady".to_string(),
        ),
    ]);
    state.insert(
        "buildrequest.machine bytecode contract".to_string(),
        machine_bytecode_contract_from_native_report(native_bytecode_report_text),
    );
    if let Some(native_bytecode_report_text) = native_bytecode_report_text {
        state.insert(
            "buildrequest.native bytecode report".to_string(),
            native_bytecode_report_text.to_string(),
        );
        state.insert(
            "buildrequest.native bytecode report fingerprint".to_string(),
            ail_artifact_fingerprint(native_bytecode_report_text),
        );
    }
    if let Some(dependency_report_text) = dependency_report_text {
        state.insert(
            "buildrequest.dependency report".to_string(),
            dependency_report_text.to_string(),
        );
        state.insert(
            "buildrequest.dependency report fingerprint".to_string(),
            ail_artifact_fingerprint(dependency_report_text),
        );
    }
    let run = run_ail_bytecode_action(&agent_bytecode, "VerifyConformanceManifest", state)?;
    if run.status != "succeeded" {
        let mut message = "ail-conformance agent VerifyConformanceManifest failed".to_string();
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

fn run_ail_lower_agent_verify_manifest(
    agent_path: &str,
    core: &ail::ail::AilCore,
    core_text: &str,
    bytecode_text: &str,
    source_artifacts: Option<&AilSourcePackageArtifacts>,
    target: Option<&str>,
) -> Result<AilLowerAgentManifestRun, String> {
    let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
    if !agent_bytecode.actions.contains_key("VerifyLowerManifest") {
        return Err("ail-lower --agent requires a VerifyLowerManifest action".to_string());
    }
    let agent_native_artifacts = if let Some(target) = target {
        compile_ail_build_agent_native_artifacts(&agent_bytecode, target)?
    } else {
        Vec::new()
    };
    let native_bytecode_report_text = if let Some(target) = target {
        Some(render_ail_lower_native_bytecode_report(
            target,
            agent_native_artifacts.as_slice(),
        )?)
    } else {
        None
    };
    let dependency_report_text = if let Some(target) = target {
        Some(render_ail_lower_dependency_report(
            target,
            agent_native_artifacts.as_slice(),
        )?)
    } else {
        None
    };
    let empty_agent_trace: &[String] = &[];
    let manifest_text = render_ail_lower_manifest(&AilLowerArtifactSet {
        source_manifest_text: source_artifacts.map(|artifacts| artifacts.manifest_text.as_str()),
        source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
        core_text,
        bytecode_text,
        native_bytecode_report_text: native_bytecode_report_text.as_deref(),
        dependency_report_text: dependency_report_text.as_deref(),
        agent_bytecode_text: Some(agent_bytecode_text.as_str()),
        agent_trace: Some(empty_agent_trace),
        agent_native_executables: agent_native_artifacts.as_slice(),
    });
    let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
    let source_package_text = source_artifacts.map(|artifacts| {
        ail_bootstrap_source_bundle_text(&artifacts.manifest_text, &artifacts.spec_text)
    });
    let source_package_fingerprint = source_package_text.as_deref().map(ail_artifact_fingerprint);
    let mut state = BTreeMap::from([
        (
            "buildrequest.id".to_string(),
            format!("{}-lower", core.package.name),
        ),
        (
            "buildrequest.developer prompt".to_string(),
            "skipped".to_string(),
        ),
        (
            "buildrequest.requirements".to_string(),
            "skipped".to_string(),
        ),
        ("buildrequest.spec".to_string(), "skipped".to_string()),
        ("buildrequest.core ir".to_string(), core_text.to_string()),
        (
            "buildrequest.core ir fingerprint".to_string(),
            ail_artifact_fingerprint(core_text),
        ),
        (
            "buildrequest.bytecode artifact".to_string(),
            format!("Verified AIL-Bytecode ({} bytes)", bytecode_text.len()),
        ),
        (
            "buildrequest.bytecode fingerprint".to_string(),
            ail_artifact_fingerprint(bytecode_text),
        ),
        ("buildrequest.artifact manifest".to_string(), manifest_text),
        (
            "buildrequest.artifact manifest fingerprint".to_string(),
            manifest_fingerprint,
        ),
        (
            "buildrequest.status".to_string(),
            "BytecodeReady".to_string(),
        ),
    ]);
    if let Some(source_package_text) = source_package_text {
        state.insert(
            "buildrequest.source package".to_string(),
            source_package_text,
        );
    }
    if let Some(source_package_fingerprint) = source_package_fingerprint {
        state.insert(
            "buildrequest.source package fingerprint".to_string(),
            source_package_fingerprint,
        );
    }
    state.insert(
        "buildrequest.machine bytecode contract".to_string(),
        machine_bytecode_contract_from_native_report(native_bytecode_report_text.as_deref()),
    );
    if let Some(native_bytecode_report_text) = native_bytecode_report_text.as_deref() {
        state.insert(
            "buildrequest.native bytecode report".to_string(),
            native_bytecode_report_text.to_string(),
        );
        state.insert(
            "buildrequest.native bytecode report fingerprint".to_string(),
            ail_artifact_fingerprint(native_bytecode_report_text),
        );
    }
    if let Some(dependency_report_text) = dependency_report_text.as_deref() {
        state.insert(
            "buildrequest.dependency report".to_string(),
            dependency_report_text.to_string(),
        );
        state.insert(
            "buildrequest.dependency report fingerprint".to_string(),
            ail_artifact_fingerprint(dependency_report_text),
        );
    }
    let run = run_ail_bytecode_action(&agent_bytecode, "VerifyLowerManifest", state)?;
    if run.status != "succeeded" {
        let mut message = "ail-lower agent VerifyLowerManifest failed".to_string();
        if let Some(failure) = run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !run.trace.is_empty() {
            message.push_str(&format!("\n{}", run.trace.join("\n")));
        }
        return Err(message);
    }
    Ok(AilLowerAgentManifestRun {
        agent_run: AilBuildAgentRun {
            bytecode: agent_bytecode,
            bytecode_text: agent_bytecode_text,
            state: run.final_state,
            trace: run.trace,
        },
        agent_native_artifacts,
        native_bytecode_report_text,
        dependency_report_text,
    })
}

fn load_ail_pass_target_core(cli_options: &CliOptions) -> Result<ail::ail::AilCore, String> {
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

fn load_ail_pass_target_source_artifacts(
    cli_options: &CliOptions,
) -> Result<Option<AilSourcePackageArtifacts>, String> {
    if cli_options.ail_core_file.is_some() {
        return Ok(None);
    }
    let target_path = cli_options
        .ail_pass_target
        .as_deref()
        .ok_or_else(|| "ail-pass requires a target package or --core-file <path>".to_string())?;
    load_ail_source_package_artifacts(target_path, "ail-pass target").map(Some)
}

fn load_ail_bytecode_or_compile_package(
    path: &str,
    context: &str,
) -> Result<(ail::ail::AilBytecodeProgram, String), String> {
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
    bytecode: &ail::ail::AilBytecodeProgram,
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
) -> Result<(ail::ail::AilBytecodeProgram, String), String> {
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
    agent_bytecode: &ail::ail::AilBytecodeProgram,
    target: &str,
) -> Result<Vec<AilNativeArtifact>, String> {
    compile_ail_native_artifacts(agent_bytecode, target, "agent")
}

fn compile_ail_pass_native_artifacts(
    pass_bytecode: &ail::ail::AilBytecodeProgram,
    target: &str,
) -> Result<Vec<AilNativeArtifact>, String> {
    compile_ail_native_artifacts(pass_bytecode, target, "pass")
}

fn compile_ail_native_artifacts(
    bytecode: &ail::ail::AilBytecodeProgram,
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
    package: &ail::ail::AilPackage,
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
    package: &ail::ail::AilPackage,
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
    core: &ail::ail::AilCore,
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
    core: &ail::ail::AilCore,
    requirements_artifact: Option<&str>,
    spec_text: Option<&str>,
    capture_prompt: Option<&str>,
    prompt_portability: AilBuildPromptPortability<'_>,
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
    if let Some(target_model) = prompt_portability.target_model {
        let base_model = prompt_portability
            .base_model
            .unwrap_or(DEFAULT_BASE_LLM_ENDPOINT);
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
            "buildrequest.base model".to_string(),
            base_model.to_string(),
        );
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

fn run_ail_build_agent_compile_native_target(
    agent_run: &mut AilBuildAgentRun,
    target: &str,
    artifact_summary: &str,
    artifact_fingerprint: &str,
) -> Result<(), String> {
    if !agent_run
        .bytecode
        .actions
        .contains_key("CompileNativeTarget")
    {
        return Err(
            "ail-build --agent native output requires a CompileNativeTarget action".to_string(),
        );
    }
    let mut compile_state = agent_run.state.clone();
    compile_state.insert(
        "buildrequest.target platform".to_string(),
        target.to_string(),
    );
    compile_state.insert(
        "buildrequest.target artifact".to_string(),
        artifact_summary.to_string(),
    );
    compile_state.insert(
        "buildrequest.target artifact fingerprint".to_string(),
        artifact_fingerprint.to_string(),
    );
    let compile_run =
        run_ail_bytecode_action(&agent_run.bytecode, "CompileNativeTarget", compile_state)?;
    if compile_run.status != "succeeded" {
        let mut message = "ail-build agent CompileNativeTarget failed".to_string();
        if let Some(failure) = compile_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !compile_run.trace.is_empty() {
            message.push_str(&format!("\n{}", compile_run.trace.join("\n")));
        }
        return Err(message);
    }
    agent_run.trace.extend(compile_run.trace);
    agent_run.state = compile_run.final_state;
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
    request: AilBuildAgentManifestVerification<'_>,
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
        request.manifest_text.to_string(),
    );
    verify_state.insert(
        "buildrequest.artifact manifest fingerprint".to_string(),
        request.manifest_fingerprint.to_string(),
    );
    if let Some(source_package_text) = request.source_package_text {
        verify_state.insert(
            "buildrequest.source package".to_string(),
            source_package_text.to_string(),
        );
    }
    if let Some(source_package_fingerprint) = request.source_package_fingerprint {
        verify_state.insert(
            "buildrequest.source package fingerprint".to_string(),
            source_package_fingerprint.to_string(),
        );
    }
    if let Some(requirements_fingerprint) = request.requirements_fingerprint {
        verify_state.insert(
            "buildrequest.requirements fingerprint".to_string(),
            requirements_fingerprint.to_string(),
        );
    }
    if let Some(spec_fingerprint) = request.spec_fingerprint {
        verify_state.insert(
            "buildrequest.spec fingerprint".to_string(),
            spec_fingerprint.to_string(),
        );
    }
    verify_state.insert(
        "buildrequest.core ir fingerprint".to_string(),
        request.core_fingerprint.to_string(),
    );
    verify_state.insert(
        "buildrequest.flow review fingerprint".to_string(),
        request.flow_review_fingerprint.to_string(),
    );
    if let Some(compiler_pass_target_fingerprint) = request.compiler_pass_target_fingerprint {
        verify_state.insert(
            "buildrequest.compiler pass target artifact fingerprint".to_string(),
            compiler_pass_target_fingerprint.to_string(),
        );
    }
    if let Some(prompt_portability_fingerprint) = request.prompt_portability_fingerprint {
        verify_state.insert(
            "buildrequest.prompt portability report fingerprint".to_string(),
            prompt_portability_fingerprint.to_string(),
        );
    }
    verify_state.insert(
        "buildrequest.machine bytecode contract".to_string(),
        machine_bytecode_contract_from_native_report(request.native_bytecode_report_text),
    );
    if let Some(native_bytecode_report_text) = request.native_bytecode_report_text {
        verify_state.insert(
            "buildrequest.native bytecode report".to_string(),
            native_bytecode_report_text.to_string(),
        );
        verify_state.insert(
            "buildrequest.native bytecode report fingerprint".to_string(),
            ail_artifact_fingerprint(native_bytecode_report_text),
        );
    }
    if let Some(dependency_report_text) = request.dependency_report_text {
        verify_state.insert(
            "buildrequest.dependency report".to_string(),
            dependency_report_text.to_string(),
        );
        verify_state.insert(
            "buildrequest.dependency report fingerprint".to_string(),
            ail_artifact_fingerprint(dependency_report_text),
        );
    }
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

struct AilCompileAgentManifestRequest<'a> {
    agent_bytecode: ail::ail::AilBytecodeProgram,
    agent_bytecode_text: String,
    package_name: &'a str,
    bytecode_text: &'a str,
    source_artifacts: Option<&'a AilSourcePackageArtifacts>,
    target_executable: &'a [u8],
    native_bytecode_report_text: &'a str,
    dependency_report_text: &'a str,
    manifest_text: &'a str,
    manifest_fingerprint: &'a str,
    target: &'a str,
}

fn run_ail_compile_agent_verify_manifest(
    request: AilCompileAgentManifestRequest<'_>,
) -> Result<AilBuildAgentRun, String> {
    let AilCompileAgentManifestRequest {
        agent_bytecode,
        agent_bytecode_text,
        package_name,
        bytecode_text,
        source_artifacts,
        target_executable,
        native_bytecode_report_text,
        dependency_report_text,
        manifest_text,
        manifest_fingerprint,
        target,
    } = request;
    if !agent_bytecode.actions.contains_key("VerifyCompileManifest") {
        return Err(
            "ail-compile --agent --artifact-dir requires a VerifyCompileManifest action"
                .to_string(),
        );
    }
    let source_package_text = source_artifacts.map(|artifacts| {
        ail_bootstrap_source_bundle_text(&artifacts.manifest_text, &artifacts.spec_text)
    });
    let source_package_fingerprint = source_package_text.as_deref().map(ail_artifact_fingerprint);
    let mut state = BTreeMap::from([
        (
            "buildrequest.id".to_string(),
            format!("{package_name}-compile"),
        ),
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
            "buildrequest.bytecode fingerprint".to_string(),
            ail_artifact_fingerprint(bytecode_text),
        ),
        (
            "buildrequest.target artifact".to_string(),
            format!("{target} executable {} bytes", target_executable.len()),
        ),
        (
            "buildrequest.target artifact fingerprint".to_string(),
            ail_artifact_fingerprint_bytes(target_executable),
        ),
        (
            "buildrequest.machine bytecode contract".to_string(),
            machine_bytecode_contract_from_native_report(Some(native_bytecode_report_text)),
        ),
        (
            "buildrequest.native bytecode report".to_string(),
            native_bytecode_report_text.to_string(),
        ),
        (
            "buildrequest.native bytecode report fingerprint".to_string(),
            ail_artifact_fingerprint(native_bytecode_report_text),
        ),
        (
            "buildrequest.dependency report".to_string(),
            dependency_report_text.to_string(),
        ),
        (
            "buildrequest.dependency report fingerprint".to_string(),
            ail_artifact_fingerprint(dependency_report_text),
        ),
        (
            "buildrequest.artifact manifest".to_string(),
            manifest_text.to_string(),
        ),
        (
            "buildrequest.artifact manifest fingerprint".to_string(),
            manifest_fingerprint.to_string(),
        ),
        (
            "buildrequest.status".to_string(),
            "BytecodeReady".to_string(),
        ),
    ]);
    if let Some(source_package_text) = source_package_text {
        state.insert(
            "buildrequest.source package".to_string(),
            source_package_text,
        );
    }
    if let Some(source_package_fingerprint) = source_package_fingerprint {
        state.insert(
            "buildrequest.source package fingerprint".to_string(),
            source_package_fingerprint,
        );
    }
    let run = run_ail_bytecode_action(&agent_bytecode, "VerifyCompileManifest", state)?;
    if run.status != "succeeded" {
        let mut message = "ail-compile agent VerifyCompileManifest failed".to_string();
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

struct AilCompileWasmContractAgentManifestRequest<'a> {
    agent_bytecode: ail::ail::AilBytecodeProgram,
    agent_bytecode_text: String,
    package_name: &'a str,
    bytecode_text: &'a str,
    source_artifacts: Option<&'a AilSourcePackageArtifacts>,
    wasm_contract_report_text: &'a str,
    dependency_report_text: &'a str,
    manifest_text: &'a str,
    manifest_fingerprint: &'a str,
    target: &'a str,
}

struct AilCompileWasmContractBundleAgentManifestRequest<'a> {
    agent_bytecode: ail::ail::AilBytecodeProgram,
    agent_bytecode_text: String,
    package_name: &'a str,
    bytecode_text: &'a str,
    source_artifacts: Option<&'a AilSourcePackageArtifacts>,
    wasm_contract_report_text: &'a str,
    dependency_report_text: &'a str,
    manifest_text: &'a str,
    manifest_fingerprint: &'a str,
    target: &'a str,
}

fn run_ail_compile_wasm_contract_agent_verify_manifest(
    request: AilCompileWasmContractAgentManifestRequest<'_>,
) -> Result<AilBuildAgentRun, String> {
    let AilCompileWasmContractAgentManifestRequest {
        agent_bytecode,
        agent_bytecode_text,
        package_name,
        bytecode_text,
        source_artifacts,
        wasm_contract_report_text,
        dependency_report_text,
        manifest_text,
        manifest_fingerprint,
        target,
    } = request;
    if !agent_bytecode.actions.contains_key("VerifyCompileManifest") {
        return Err(
            "ail-compile --agent --artifact-dir requires a VerifyCompileManifest action"
                .to_string(),
        );
    }
    let source_package_text = source_artifacts.map(|artifacts| {
        ail_bootstrap_source_bundle_text(&artifacts.manifest_text, &artifacts.spec_text)
    });
    let source_package_fingerprint = source_package_text.as_deref().map(ail_artifact_fingerprint);
    let mut state = BTreeMap::from([
        (
            "buildrequest.id".to_string(),
            format!("{package_name}-wasm-contract-compile"),
        ),
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
            "buildrequest.bytecode fingerprint".to_string(),
            ail_artifact_fingerprint(bytecode_text),
        ),
        (
            "buildrequest.target artifact".to_string(),
            wasm_contract_report_text.to_string(),
        ),
        (
            "buildrequest.target artifact fingerprint".to_string(),
            ail_artifact_fingerprint(wasm_contract_report_text),
        ),
        (
            "buildrequest.machine bytecode contract".to_string(),
            wasm_contract_machine_bytecode_manifest_contract_line(target),
        ),
        (
            "buildrequest.native bytecode report".to_string(),
            wasm_contract_report_text.to_string(),
        ),
        (
            "buildrequest.native bytecode report fingerprint".to_string(),
            ail_artifact_fingerprint(wasm_contract_report_text),
        ),
        (
            "buildrequest.dependency report".to_string(),
            dependency_report_text.to_string(),
        ),
        (
            "buildrequest.dependency report fingerprint".to_string(),
            ail_artifact_fingerprint(dependency_report_text),
        ),
        (
            "buildrequest.artifact manifest".to_string(),
            manifest_text.to_string(),
        ),
        (
            "buildrequest.artifact manifest fingerprint".to_string(),
            manifest_fingerprint.to_string(),
        ),
        (
            "buildrequest.status".to_string(),
            "BytecodeReady".to_string(),
        ),
    ]);
    if let Some(source_package_text) = source_package_text {
        state.insert(
            "buildrequest.source package".to_string(),
            source_package_text,
        );
    }
    if let Some(source_package_fingerprint) = source_package_fingerprint {
        state.insert(
            "buildrequest.source package fingerprint".to_string(),
            source_package_fingerprint,
        );
    }
    let run = run_ail_bytecode_action(&agent_bytecode, "VerifyCompileManifest", state)?;
    if run.status != "succeeded" {
        let mut message = "ail-compile agent VerifyCompileManifest failed".to_string();
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

fn run_ail_compile_wasm_contract_bundle_agent_verify_manifest(
    request: AilCompileWasmContractBundleAgentManifestRequest<'_>,
) -> Result<AilBuildAgentRun, String> {
    let AilCompileWasmContractBundleAgentManifestRequest {
        agent_bytecode,
        agent_bytecode_text,
        package_name,
        bytecode_text,
        source_artifacts,
        wasm_contract_report_text,
        dependency_report_text,
        manifest_text,
        manifest_fingerprint,
        target,
    } = request;
    if !agent_bytecode
        .actions
        .contains_key("VerifyCompileBundleManifest")
    {
        return Err(
            "ail-compile --all-actions --agent requires a VerifyCompileBundleManifest action"
                .to_string(),
        );
    }
    let source_package_text = source_artifacts.map(|artifacts| {
        ail_bootstrap_source_bundle_text(&artifacts.manifest_text, &artifacts.spec_text)
    });
    let source_package_fingerprint = source_package_text.as_deref().map(ail_artifact_fingerprint);
    let mut state = BTreeMap::from([
        (
            "buildrequest.id".to_string(),
            format!("{package_name}-wasm-contract-bundle-compile"),
        ),
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
            "buildrequest.bytecode fingerprint".to_string(),
            ail_artifact_fingerprint(bytecode_text),
        ),
        (
            "buildrequest.target artifact".to_string(),
            wasm_contract_report_text.to_string(),
        ),
        (
            "buildrequest.target artifact fingerprint".to_string(),
            ail_artifact_fingerprint(wasm_contract_report_text),
        ),
        (
            "buildrequest.machine bytecode contract".to_string(),
            wasm_contract_machine_bytecode_manifest_contract_line(target),
        ),
        (
            "buildrequest.native bytecode report".to_string(),
            wasm_contract_report_text.to_string(),
        ),
        (
            "buildrequest.native bytecode report fingerprint".to_string(),
            ail_artifact_fingerprint(wasm_contract_report_text),
        ),
        (
            "buildrequest.dependency report".to_string(),
            dependency_report_text.to_string(),
        ),
        (
            "buildrequest.dependency report fingerprint".to_string(),
            ail_artifact_fingerprint(dependency_report_text),
        ),
        (
            "buildrequest.artifact manifest".to_string(),
            manifest_text.to_string(),
        ),
        (
            "buildrequest.artifact manifest fingerprint".to_string(),
            manifest_fingerprint.to_string(),
        ),
        (
            "buildrequest.status".to_string(),
            "BytecodeReady".to_string(),
        ),
    ]);
    if let Some(source_package_text) = source_package_text {
        state.insert(
            "buildrequest.source package".to_string(),
            source_package_text,
        );
    }
    if let Some(source_package_fingerprint) = source_package_fingerprint {
        state.insert(
            "buildrequest.source package fingerprint".to_string(),
            source_package_fingerprint,
        );
    }
    let run = run_ail_bytecode_action(&agent_bytecode, "VerifyCompileBundleManifest", state)?;
    if run.status != "succeeded" {
        let mut message = "ail-compile agent VerifyCompileBundleManifest failed".to_string();
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

struct AilCompileBundleAgentManifestRequest<'a> {
    agent_bytecode: ail::ail::AilBytecodeProgram,
    agent_bytecode_text: String,
    package_name: &'a str,
    bytecode_text: &'a str,
    source_artifacts: Option<&'a AilSourcePackageArtifacts>,
    target: &'a str,
    target_executables: &'a [AilNativeArtifact],
    native_bytecode_report_text: &'a str,
    dependency_report_text: &'a str,
    manifest_text: &'a str,
    manifest_fingerprint: &'a str,
}

fn run_ail_compile_bundle_agent_verify_manifest(
    request: AilCompileBundleAgentManifestRequest<'_>,
) -> Result<AilBuildAgentRun, String> {
    let AilCompileBundleAgentManifestRequest {
        agent_bytecode,
        agent_bytecode_text,
        package_name,
        bytecode_text,
        source_artifacts,
        target,
        target_executables,
        native_bytecode_report_text,
        dependency_report_text,
        manifest_text,
        manifest_fingerprint,
    } = request;
    if !agent_bytecode
        .actions
        .contains_key("VerifyCompileBundleManifest")
    {
        return Err(
            "ail-compile --all-actions --agent requires a VerifyCompileBundleManifest action"
                .to_string(),
        );
    }
    let target_fingerprint =
        native_artifact_fingerprint_text(target_executables).unwrap_or_default();
    let source_package_text = source_artifacts.map(|artifacts| {
        ail_bootstrap_source_bundle_text(&artifacts.manifest_text, &artifacts.spec_text)
    });
    let source_package_fingerprint = source_package_text.as_deref().map(ail_artifact_fingerprint);
    let mut state = BTreeMap::from([
        (
            "buildrequest.id".to_string(),
            format!("{package_name}-compile-bundle"),
        ),
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
            "buildrequest.bytecode fingerprint".to_string(),
            ail_artifact_fingerprint(bytecode_text),
        ),
        (
            "buildrequest.target artifact".to_string(),
            format!("{target} bundle {} executables", target_executables.len()),
        ),
        (
            "buildrequest.target artifact fingerprint".to_string(),
            target_fingerprint,
        ),
        (
            "buildrequest.machine bytecode contract".to_string(),
            machine_bytecode_contract_from_native_report(Some(native_bytecode_report_text)),
        ),
        (
            "buildrequest.native bytecode report".to_string(),
            native_bytecode_report_text.to_string(),
        ),
        (
            "buildrequest.native bytecode report fingerprint".to_string(),
            ail_artifact_fingerprint(native_bytecode_report_text),
        ),
        (
            "buildrequest.dependency report".to_string(),
            dependency_report_text.to_string(),
        ),
        (
            "buildrequest.dependency report fingerprint".to_string(),
            ail_artifact_fingerprint(dependency_report_text),
        ),
        (
            "buildrequest.artifact manifest".to_string(),
            manifest_text.to_string(),
        ),
        (
            "buildrequest.artifact manifest fingerprint".to_string(),
            manifest_fingerprint.to_string(),
        ),
        (
            "buildrequest.status".to_string(),
            "BytecodeReady".to_string(),
        ),
    ]);
    if let Some(source_package_text) = source_package_text {
        state.insert(
            "buildrequest.source package".to_string(),
            source_package_text,
        );
    }
    if let Some(source_package_fingerprint) = source_package_fingerprint {
        state.insert(
            "buildrequest.source package fingerprint".to_string(),
            source_package_fingerprint,
        );
    }
    let run = run_ail_bytecode_action(&agent_bytecode, "VerifyCompileBundleManifest", state)?;
    if run.status != "succeeded" {
        let mut message = "ail-compile agent VerifyCompileBundleManifest failed".to_string();
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

struct AilBootstrapAgentManifestRequest<'a> {
    agent_bytecode: ail::ail::AilBytecodeProgram,
    agent_bytecode_text: String,
    package_name: &'a str,
    toolchain_source_manifest_text: &'a str,
    toolchain_source_spec_text: &'a str,
    toolchain_core_text: &'a str,
    toolchain_bytecode_text: &'a str,
    compiler_pass_source_manifest_text: &'a str,
    compiler_pass_source_spec_text: &'a str,
    compiler_pass_core_text: &'a str,
    compiler_pass_bytecode_text: &'a str,
    toolchain_pass_output_core_text: &'a str,
    toolchain_pass_trace_text: &'a str,
    fixed_point_report_text: &'a str,
    native_bytecode_report_text: &'a str,
    host_boundary_report_text: &'a str,
    dependency_report_text: &'a str,
    handoff_report_text: &'a str,
    toolchain_conformance_report: &'a str,
    compiler_pass_conformance_report: &'a str,
    target_artifacts: &'a [AilNativeArtifact],
    compiler_pass_artifacts: &'a [AilNativeArtifact],
    manifest_text: &'a str,
    manifest_fingerprint: &'a str,
}

fn run_ail_bootstrap_agent_verify_manifest(
    request: AilBootstrapAgentManifestRequest<'_>,
) -> Result<AilBuildAgentRun, String> {
    let AilBootstrapAgentManifestRequest {
        agent_bytecode,
        agent_bytecode_text,
        package_name,
        toolchain_source_manifest_text,
        toolchain_source_spec_text,
        toolchain_core_text,
        toolchain_bytecode_text,
        compiler_pass_source_manifest_text,
        compiler_pass_source_spec_text,
        compiler_pass_core_text,
        compiler_pass_bytecode_text,
        toolchain_pass_output_core_text,
        toolchain_pass_trace_text,
        fixed_point_report_text,
        native_bytecode_report_text,
        host_boundary_report_text,
        dependency_report_text,
        handoff_report_text,
        toolchain_conformance_report,
        compiler_pass_conformance_report,
        target_artifacts,
        compiler_pass_artifacts,
        manifest_text,
        manifest_fingerprint,
    } = request;
    if !agent_bytecode
        .actions
        .contains_key("VerifyBootstrapManifest")
    {
        return Err("ail-bootstrap --agent requires a VerifyBootstrapManifest action".to_string());
    }
    let source_report = format!(
        "toolchain-agent:\n{}\ncompiler-pass:\n{}",
        ail_bootstrap_source_bundle_text(
            toolchain_source_manifest_text,
            toolchain_source_spec_text
        ),
        ail_bootstrap_source_bundle_text(
            compiler_pass_source_manifest_text,
            compiler_pass_source_spec_text
        )
    );
    let core_report = format!(
        "toolchain-agent:\n{toolchain_core_text}\ntoolchain-agent-pass-output:\n{toolchain_pass_output_core_text}\ncompiler-pass:\n{compiler_pass_core_text}"
    );
    let conformance_report = format!(
        "toolchain-agent:\n{toolchain_conformance_report}\ncompiler-pass:\n{compiler_pass_conformance_report}"
    );
    let state = BTreeMap::from([
        (
            "buildrequest.id".to_string(),
            format!("{package_name}-bootstrap"),
        ),
        (
            "buildrequest.status".to_string(),
            "BytecodeReady".to_string(),
        ),
        (
            "buildrequest.source package".to_string(),
            source_report.clone(),
        ),
        (
            "buildrequest.source package fingerprint".to_string(),
            ail_artifact_fingerprint(&source_report),
        ),
        ("buildrequest.core ir".to_string(), core_report.clone()),
        (
            "buildrequest.core ir fingerprint".to_string(),
            ail_artifact_fingerprint(&core_report),
        ),
        (
            "buildrequest.bytecode fingerprint".to_string(),
            ail_artifact_fingerprint(toolchain_bytecode_text),
        ),
        (
            "buildrequest.compiler pass fingerprint".to_string(),
            ail_artifact_fingerprint(compiler_pass_bytecode_text),
        ),
        (
            "buildrequest.compiler pass trace".to_string(),
            toolchain_pass_trace_text.to_string(),
        ),
        (
            "buildrequest.fixed point report".to_string(),
            fixed_point_report_text.to_string(),
        ),
        (
            "buildrequest.fixed point report fingerprint".to_string(),
            ail_artifact_fingerprint(fixed_point_report_text),
        ),
        (
            "buildrequest.machine bytecode contract".to_string(),
            machine_bytecode_contract_from_native_report(Some(native_bytecode_report_text)),
        ),
        (
            "buildrequest.native bytecode report".to_string(),
            native_bytecode_report_text.to_string(),
        ),
        (
            "buildrequest.native bytecode report fingerprint".to_string(),
            ail_artifact_fingerprint(native_bytecode_report_text),
        ),
        (
            "buildrequest.host boundary report".to_string(),
            host_boundary_report_text.to_string(),
        ),
        (
            "buildrequest.host boundary report fingerprint".to_string(),
            ail_artifact_fingerprint(host_boundary_report_text),
        ),
        (
            "buildrequest.dependency report".to_string(),
            dependency_report_text.to_string(),
        ),
        (
            "buildrequest.dependency report fingerprint".to_string(),
            ail_artifact_fingerprint(dependency_report_text),
        ),
        (
            "buildrequest.handoff report".to_string(),
            handoff_report_text.to_string(),
        ),
        (
            "buildrequest.handoff report fingerprint".to_string(),
            ail_artifact_fingerprint(handoff_report_text),
        ),
        (
            "buildrequest.conformance report".to_string(),
            conformance_report.clone(),
        ),
        (
            "buildrequest.conformance report fingerprint".to_string(),
            ail_artifact_fingerprint(&conformance_report),
        ),
        (
            "buildrequest.target artifact fingerprint".to_string(),
            native_artifact_fingerprint_text(target_artifacts).unwrap_or_default(),
        ),
        (
            "buildrequest.compiler pass target artifact fingerprint".to_string(),
            native_artifact_fingerprint_text(compiler_pass_artifacts).unwrap_or_default(),
        ),
        (
            "buildrequest.artifact manifest".to_string(),
            manifest_text.to_string(),
        ),
        (
            "buildrequest.artifact manifest fingerprint".to_string(),
            manifest_fingerprint.to_string(),
        ),
    ]);
    let run = run_ail_bytecode_action(&agent_bytecode, "VerifyBootstrapManifest", state)?;
    if run.status != "succeeded" {
        let mut message = "ail-bootstrap agent VerifyBootstrapManifest failed".to_string();
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

fn native_artifact_fingerprint_text(artifacts: &[AilNativeArtifact]) -> Option<String> {
    if artifacts.is_empty() {
        return None;
    }
    Some(
        artifacts
            .iter()
            .map(|artifact| ail_artifact_fingerprint_bytes(&artifact.bytes))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn native_machine_bytecode_report_header(
    report_title: &str,
    target_name: &str,
) -> Result<Vec<String>, String> {
    let (container, format) = native_machine_bytecode_contract_parts(target_name)?;
    Ok(vec![
        report_title.to_string(),
        format!("target {target_name}"),
        "bytecode-level machine".to_string(),
        format!("bytecode-container {container}"),
        format!("bytecode-format {format}"),
    ])
}

fn native_machine_bytecode_contract_parts(
    target_name: &str,
) -> Result<(&'static str, &'static str), String> {
    match target_name {
        "linux-x86_64-elf" => Ok(("linux-elf-executable", "elf64-little-x86_64-executable")),
        _ => Err(format!("unsupported native bytecode target {target_name}")),
    }
}

fn native_machine_bytecode_manifest_contract_line(target_name: &str) -> String {
    let (container, format) = native_machine_bytecode_contract_parts(target_name)
        .unwrap_or(("unsupported-native-executable", "unsupported-native-format"));
    format!(
        "machine-bytecode-contract {target_name} bytecode-level machine bytecode-container {container} bytecode-format {format}"
    )
}

fn machine_bytecode_contract_from_native_report(
    native_bytecode_report_text: Option<&str>,
) -> String {
    native_bytecode_report_text
        .and_then(|report| {
            report
                .lines()
                .find_map(|line| line.strip_prefix("target ").map(str::trim))
        })
        .map(native_machine_bytecode_manifest_contract_line)
        .unwrap_or_else(|| "none".to_string())
}

fn native_machine_bytecode_manifest_contract_line_from_artifacts(
    artifact_groups: &[&[AilNativeArtifact]],
) -> Option<String> {
    artifact_groups
        .iter()
        .find_map(|artifacts| artifacts.first())
        .map(|artifact| native_machine_bytecode_manifest_contract_line(&artifact.target_name))
}

fn render_ail_bootstrap_native_bytecode_report(
    target_name: &str,
    toolchain_artifacts: &[AilNativeArtifact],
    compiler_pass_artifacts: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines =
        native_machine_bytecode_report_header("AIL-Bootstrap-Native-Bytecode:", target_name)?;
    for (role, artifacts) in [
        ("toolchain-agent-target", toolchain_artifacts),
        ("compiler-pass-target", compiler_pass_artifacts),
        ("agent-target", agent_artifacts),
    ] {
        for artifact in artifacts {
            if artifact.target_name != target_name {
                return Err(format!(
                    "native bytecode artifact {} targets {}, expected {target_name}",
                    artifact.file_name, artifact.target_name
                ));
            }
            lines.push(format!(
                "machine-bytecode {role} {} {} {} {} bytes {}",
                artifact.target_name,
                artifact.file_name,
                native_machine_bytecode_identity(&artifact.bytes)?,
                ail_artifact_fingerprint_bytes(&artifact.bytes),
                artifact.bytes.len()
            ));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_bootstrap_host_boundary_report(
    target_name: &str,
    toolchain_artifacts: &[AilNativeArtifact],
    compiler_pass_artifacts: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Bootstrap-Host-Boundary:".to_string(),
        format!("target {target_name}"),
        "no-host-backend-source true".to_string(),
        "bootstrap-language rust stage0-scaffold-only".to_string(),
        "generated-host-language-source none".to_string(),
        "forbidden-host-source-suffixes .rs .c .cc .cpp .h .hpp .py .js .ts .go .java .ll .bc .wasm".to_string(),
        "ail-source toolchain-agent.source.ail-package.md".to_string(),
        "ail-source toolchain-agent.source.ail-spec.md".to_string(),
        "ail-source compiler-pass.source.ail-package.md".to_string(),
        "ail-source compiler-pass.source.ail-spec.md".to_string(),
        "ail-core toolchain-agent.checked.ail-core.txt".to_string(),
        "ail-core compiler-pass.checked.ail-core.txt".to_string(),
        "ail-core toolchain-agent.pass-output.ail-core.txt".to_string(),
        "ail-bytecode toolchain-agent.ailbc.json".to_string(),
        "ail-bytecode compiler-pass.ailbc.json".to_string(),
        "ail-bytecode agent.ailbc.json".to_string(),
        "report bootstrap-fixed-point-report.txt".to_string(),
        "report bootstrap-native-bytecode-report.txt".to_string(),
        "report toolchain-agent-conformance-report.txt".to_string(),
        "report compiler-pass-conformance-report.txt".to_string(),
    ];
    for artifacts in [
        toolchain_artifacts,
        compiler_pass_artifacts,
        agent_artifacts,
    ] {
        for artifact in artifacts {
            if artifact.target_name != target_name {
                return Err(format!(
                    "host boundary artifact {} targets {}, expected {target_name}",
                    artifact.file_name, artifact.target_name
                ));
            }
            lines.push(format!(
                "machine-bytecode {} {}",
                artifact.file_name,
                native_machine_bytecode_identity(&artifact.bytes)?
            ));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_bootstrap_dependency_report(
    target_name: &str,
    toolchain_artifacts: &[AilNativeArtifact],
    compiler_pass_artifacts: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Bootstrap-Dependency-Report:".to_string(),
        format!("target {target_name}"),
        "host-language-runtime none".to_string(),
        "dynamic-linker none".to_string(),
        "shared-libraries none".to_string(),
        "library-dependencies none".to_string(),
        "linker-invocation none".to_string(),
        "runtime-abi linux-syscall-argv-key-value".to_string(),
    ];
    for artifacts in [
        toolchain_artifacts,
        compiler_pass_artifacts,
        agent_artifacts,
    ] {
        for artifact in artifacts {
            if artifact.target_name != target_name {
                return Err(format!(
                    "dependency artifact {} targets {}, expected {target_name}",
                    artifact.file_name, artifact.target_name
                ));
            }
            lines.push(format!(
                "machine-bytecode-dependency {} {}",
                artifact.file_name,
                native_elf_dependency_identity(&artifact.bytes)?
            ));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_bootstrap_handoff_report(
    target_name: &str,
    toolchain_artifacts: &[AilNativeArtifact],
    compiler_pass_artifacts: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Bootstrap-Handoff-Report:".to_string(),
        format!("target {target_name}"),
        "runtime-abi linux-syscall-argv-key-value".to_string(),
    ];
    append_bootstrap_handoff_role(
        &mut lines,
        target_name,
        "toolchain-agent",
        toolchain_artifacts,
    )?;
    append_bootstrap_handoff_role(
        &mut lines,
        target_name,
        "compiler-pass",
        compiler_pass_artifacts,
    )?;
    append_bootstrap_handoff_role(&mut lines, target_name, "agent", agent_artifacts)?;
    Ok(format!("{}\n", lines.join("\n")))
}

struct BootstrapHandoffCase {
    trace_marker: &'static str,
    args: &'static [&'static str],
}

fn append_bootstrap_handoff_role(
    lines: &mut Vec<String>,
    target_name: &str,
    file_prefix: &str,
    artifacts: &[AilNativeArtifact],
) -> Result<(), String> {
    for artifact in artifacts {
        if artifact.target_name != target_name {
            return Err(format!(
                "native handoff artifact {} targets {}, expected {target_name}",
                artifact.file_name, artifact.target_name
            ));
        }
        let action_name = native_handoff_action_name(file_prefix, &artifact.file_name)?;
        let handoff_case = bootstrap_handoff_case(action_name)?;
        append_bootstrap_handoff_action(
            lines,
            artifact,
            handoff_case.trace_marker,
            handoff_case.args,
        )?;
    }
    lines.push(format!(
        "handoff-native-role {file_prefix} all-actions ok count {}",
        artifacts.len()
    ));
    Ok(())
}

fn native_handoff_action_name<'a>(
    file_prefix: &str,
    file_name: &'a str,
) -> Result<&'a str, String> {
    let prefix = format!("{file_prefix}-");
    file_name
        .strip_prefix(&prefix)
        .and_then(|name| name.strip_suffix(".elf"))
        .ok_or_else(|| format!("native handoff artifact {file_name} does not use {prefix}*.elf"))
}

fn bootstrap_handoff_case(action_name: &str) -> Result<BootstrapHandoffCase, String> {
    match action_name {
        "AcceptCompilerPassOutput" => Ok(BootstrapHandoffCase {
            trace_marker: "CompilerPassOutputAccepted",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=SpecCaptured",
                "buildrequest.requirements=checked",
                "buildrequest.spec=checked",
                "buildrequest.core ir=checked",
                "buildrequest.compiler pass artifact=pass-bytecode",
                "buildrequest.compiler pass fingerprint=fnv64:pass-bytecode",
                "buildrequest.compiler pass trace=checked",
            ],
        }),
        "AcceptCoreIR" => Ok(BootstrapHandoffCase {
            trace_marker: "CoreIrAccepted",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=SpecCaptured",
                "buildrequest.requirements=checked",
                "buildrequest.spec=checked",
                "buildrequest.core ir=checked",
            ],
        }),
        "AcceptSpecDraft" => Ok(BootstrapHandoffCase {
            trace_marker: "SpecDraftAccepted",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=RequirementsCaptured",
                "buildrequest.requirements=checked",
                "buildrequest.spec=checked",
            ],
        }),
        "CaptureRequirements" => Ok(BootstrapHandoffCase {
            trace_marker: "RequirementsCaptured",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.developer prompt=Build a native AIL toolchain",
            ],
        }),
        "CompareAgentPromptPortability" => Ok(BootstrapHandoffCase {
            trace_marker: "AgentPromptPortabilityCompared",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.base model=unsloth/Qwen3.6-35B-A3B-GGUF:UD-Q4_K_XL",
                "buildrequest.target model=local-port",
                "buildrequest.requirements=checked",
            ],
        }),
        "CompileApplication" => Ok(BootstrapHandoffCase {
            trace_marker: "ApplicationBytecodeCompiled",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=SpecCaptured",
                "buildrequest.requirements=checked",
                "buildrequest.spec=checked",
            ],
        }),
        "CompileNativeTarget" => Ok(BootstrapHandoffCase {
            trace_marker: "NativeTargetCompiled",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=BytecodeReady",
                "buildrequest.bytecode artifact=verified",
                "buildrequest.bytecode fingerprint=fnv64:handoff-bytecode",
                "buildrequest.target platform=linux-x86_64-elf",
                "buildrequest.target artifact=elf-bytes",
                "buildrequest.target artifact fingerprint=fnv64:handoff-target",
            ],
        }),
        "InferReadPermissions" => Ok(BootstrapHandoffCase {
            trace_marker: "ReadPermissionAdded",
            args: &[
                "input graph=checked-ail-core",
                "package policy=permission-inference",
            ],
        }),
        "PrepareSpecDraft" => Ok(BootstrapHandoffCase {
            trace_marker: "SpecDraftPrepared",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=RequirementsCaptured",
                "buildrequest.requirements=checked",
            ],
        }),
        "VerifyBootstrapManifest" => Ok(BootstrapHandoffCase {
            trace_marker: "BootstrapManifestVerified",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=BytecodeReady",
                "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
                "buildrequest.artifact manifest=manifest",
                "buildrequest.artifact manifest fingerprint=fnv64:manifest",
                "buildrequest.source package=source",
                "buildrequest.source package fingerprint=fnv64:source",
                "buildrequest.core ir=core",
                "buildrequest.core ir fingerprint=fnv64:core",
                "buildrequest.bytecode fingerprint=fnv64:bytecode",
                "buildrequest.compiler pass fingerprint=fnv64:pass",
                "buildrequest.compiler pass trace=trace",
                "buildrequest.fixed point report=fixed-point",
                "buildrequest.fixed point report fingerprint=fnv64:fixed-point",
                "buildrequest.conformance report=conformance",
                "buildrequest.conformance report fingerprint=fnv64:conformance",
                "buildrequest.native bytecode report=native-bytecode",
                "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
                "buildrequest.host boundary report=host-boundary",
                "buildrequest.host boundary report fingerprint=fnv64:host-boundary",
                "buildrequest.dependency report=dependencies",
                "buildrequest.dependency report fingerprint=fnv64:dependencies",
                "buildrequest.handoff report=handoff",
                "buildrequest.handoff report fingerprint=fnv64:handoff",
                "buildrequest.target artifact fingerprint=fnv64:target",
                "buildrequest.compiler pass target artifact fingerprint=fnv64:pass-target",
            ],
        }),
        "VerifyBuildManifest" => Ok(BootstrapHandoffCase {
            trace_marker: "BuildManifestVerified",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=BytecodeReady",
                "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
                "buildrequest.artifact manifest=manifest",
                "buildrequest.artifact manifest fingerprint=fnv64:manifest",
                "buildrequest.source package=source",
                "buildrequest.source package fingerprint=fnv64:source",
                "buildrequest.requirements fingerprint=fnv64:requirements",
                "buildrequest.spec fingerprint=fnv64:spec",
                "buildrequest.core ir fingerprint=fnv64:core",
                "buildrequest.bytecode fingerprint=fnv64:bytecode",
                "buildrequest.target artifact fingerprint=fnv64:target",
                "buildrequest.compiler pass target artifact fingerprint=fnv64:pass-target",
                "buildrequest.prompt portability report fingerprint=fnv64:prompt",
                "buildrequest.native bytecode report=native-bytecode",
                "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
            ],
        }),
        "VerifyBytecodeArtifact" => Ok(BootstrapHandoffCase {
            trace_marker: "BytecodeArtifactVerified",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=BytecodeReady",
                "buildrequest.bytecode artifact=bytecode",
                "buildrequest.bytecode fingerprint=fnv64:bytecode",
            ],
        }),
        "VerifyCompileBundleManifest" => Ok(BootstrapHandoffCase {
            trace_marker: "CompileBundleManifestVerified",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=BytecodeReady",
                "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
                "buildrequest.artifact manifest=manifest",
                "buildrequest.artifact manifest fingerprint=fnv64:manifest",
                "buildrequest.bytecode fingerprint=fnv64:bytecode",
                "buildrequest.source package=source",
                "buildrequest.source package fingerprint=fnv64:source",
                "buildrequest.target artifact=target",
                "buildrequest.target artifact fingerprint=fnv64:target",
                "buildrequest.native bytecode report=native-bytecode",
                "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
            ],
        }),
        "VerifyCompileManifest" => Ok(BootstrapHandoffCase {
            trace_marker: "CompileManifestVerified",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=BytecodeReady",
                "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
                "buildrequest.artifact manifest=manifest",
                "buildrequest.artifact manifest fingerprint=fnv64:manifest",
                "buildrequest.bytecode fingerprint=fnv64:bytecode",
                "buildrequest.source package=source",
                "buildrequest.source package fingerprint=fnv64:source",
                "buildrequest.target artifact=target",
                "buildrequest.target artifact fingerprint=fnv64:target",
                "buildrequest.native bytecode report=native-bytecode",
                "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
            ],
        }),
        "VerifyConformanceManifest" => Ok(BootstrapHandoffCase {
            trace_marker: "ConformanceManifestVerified",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=BytecodeReady",
                "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
                "buildrequest.conformance report=conformance",
                "buildrequest.conformance report fingerprint=fnv64:conformance",
                "buildrequest.artifact manifest=manifest",
                "buildrequest.artifact manifest fingerprint=fnv64:manifest",
                "buildrequest.native bytecode report=native-bytecode",
                "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
                "buildrequest.dependency report=dependencies",
                "buildrequest.dependency report fingerprint=fnv64:dependencies",
            ],
        }),
        "VerifyLowerManifest" => Ok(BootstrapHandoffCase {
            trace_marker: "LowerManifestVerified",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=BytecodeReady",
                "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
                "buildrequest.core ir=core",
                "buildrequest.core ir fingerprint=fnv64:core",
                "buildrequest.source package=source",
                "buildrequest.source package fingerprint=fnv64:source",
                "buildrequest.bytecode artifact=bytecode",
                "buildrequest.bytecode fingerprint=fnv64:bytecode",
                "buildrequest.artifact manifest=manifest",
                "buildrequest.artifact manifest fingerprint=fnv64:manifest",
                "buildrequest.native bytecode report=native-bytecode",
                "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
                "buildrequest.dependency report=dependencies",
                "buildrequest.dependency report fingerprint=fnv64:dependencies",
            ],
        }),
        "VerifyPassManifest" => Ok(BootstrapHandoffCase {
            trace_marker: "PassManifestVerified",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=PassApplied",
                "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
                "buildrequest.artifact manifest=manifest",
                "buildrequest.artifact manifest fingerprint=fnv64:manifest",
                "buildrequest.compiler pass source package=pass-source",
                "buildrequest.compiler pass source package fingerprint=fnv64:pass-source",
                "buildrequest.compiler pass fingerprint=fnv64:pass",
                "buildrequest.source package=source",
                "buildrequest.source package fingerprint=fnv64:source",
                "buildrequest.native bytecode report=native-bytecode",
                "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
                "buildrequest.dependency report=dependencies",
                "buildrequest.dependency report fingerprint=fnv64:dependencies",
            ],
        }),
        "VerifyTargetArtifact" => Ok(BootstrapHandoffCase {
            trace_marker: "TargetArtifactVerified",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=BytecodeReady",
                "buildrequest.target artifact=target",
                "buildrequest.target artifact fingerprint=fnv64:target",
            ],
        }),
        _ => Err(format!(
            "missing native bootstrap handoff argv case for action {action_name}"
        )),
    }
}

fn append_bootstrap_handoff_action(
    lines: &mut Vec<String>,
    artifact: &AilNativeArtifact,
    trace_marker: &str,
    args: &[&str],
) -> Result<(), String> {
    let (stdout, stderr) = run_bootstrap_handoff_native_action(artifact, args)?;
    if !stdout.contains(trace_marker) && !stderr.contains(trace_marker) {
        return Err(format!(
            "native handoff artifact {} did not emit trace {trace_marker}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            artifact.file_name
        ));
    }
    lines.push(format!(
        "handoff-native-action {} ok trace {}",
        artifact.file_name, trace_marker
    ));
    lines.push(format!(
        "handoff-stdout {} {}",
        artifact.file_name,
        ail_artifact_fingerprint(&stdout)
    ));
    lines.push(format!(
        "handoff-stderr {} {}",
        artifact.file_name,
        ail_artifact_fingerprint(&stderr)
    ));
    Ok(())
}

fn run_bootstrap_handoff_native_action(
    artifact: &AilNativeArtifact,
    args: &[&str],
) -> Result<(String, String), String> {
    let executable_path = env::temp_dir().join(format!(
        "ail-bootstrap-handoff-{}-{}",
        std::process::id(),
        artifact.file_name
    ));
    fs::write(&executable_path, &artifact.bytes).map_err(|error| {
        format!(
            "failed to write native handoff artifact {}: {error}",
            artifact.file_name
        )
    })?;
    let executable_path_text = executable_path.to_string_lossy().into_owned();
    if let Err(error) = set_native_executable_permissions(&executable_path_text) {
        let _ = fs::remove_file(&executable_path);
        return Err(error);
    }
    let output = match Command::new(&executable_path).args(args).output() {
        Ok(output) => output,
        Err(error) => {
            let _ = fs::remove_file(&executable_path);
            return Err(format!(
                "failed to run native handoff artifact {}: {error}",
                artifact.file_name
            ));
        }
    };
    let _ = fs::remove_file(&executable_path);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        return Err(format!(
            "native handoff artifact {} exited with {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            artifact.file_name, output.status
        ));
    }
    Ok((stdout, stderr))
}

fn render_ail_lower_native_bytecode_report(
    target_name: &str,
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines =
        native_machine_bytecode_report_header("AIL-Lower-Native-Bytecode:", target_name)?;
    for artifact in agent_artifacts {
        if artifact.target_name != target_name {
            return Err(format!(
                "native bytecode artifact {} targets {}, expected {target_name}",
                artifact.file_name, artifact.target_name
            ));
        }
        lines.push(format!(
            "machine-bytecode agent-target {} {} {} {} bytes {}",
            artifact.target_name,
            artifact.file_name,
            native_machine_bytecode_identity(&artifact.bytes)?,
            ail_artifact_fingerprint_bytes(&artifact.bytes),
            artifact.bytes.len()
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_lower_dependency_report(
    target_name: &str,
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Lower-Dependency-Report:".to_string(),
        format!("target {target_name}"),
        "host-language-runtime none".to_string(),
        "dynamic-linker none".to_string(),
        "shared-libraries none".to_string(),
        "library-dependencies none".to_string(),
        "linker-invocation none".to_string(),
        "runtime-abi linux-syscall-argv-key-value".to_string(),
    ];
    for artifact in agent_artifacts {
        if artifact.target_name != target_name {
            return Err(format!(
                "dependency artifact {} targets {}, expected {target_name}",
                artifact.file_name, artifact.target_name
            ));
        }
        lines.push(format!(
            "machine-bytecode-dependency {} {}",
            artifact.file_name,
            native_elf_dependency_identity(&artifact.bytes)?
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_conformance_native_bytecode_report(
    target_name: &str,
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines =
        native_machine_bytecode_report_header("AIL-Conformance-Native-Bytecode:", target_name)?;
    for artifact in agent_artifacts {
        if artifact.target_name != target_name {
            return Err(format!(
                "native bytecode artifact {} targets {}, expected {target_name}",
                artifact.file_name, artifact.target_name
            ));
        }
        lines.push(format!(
            "machine-bytecode agent-target {} {} {} {} bytes {}",
            artifact.target_name,
            artifact.file_name,
            native_machine_bytecode_identity(&artifact.bytes)?,
            ail_artifact_fingerprint_bytes(&artifact.bytes),
            artifact.bytes.len()
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_conformance_dependency_report(
    target_name: &str,
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Conformance-Dependency-Report:".to_string(),
        format!("target {target_name}"),
        "host-language-runtime none".to_string(),
        "dynamic-linker none".to_string(),
        "shared-libraries none".to_string(),
        "library-dependencies none".to_string(),
        "linker-invocation none".to_string(),
        "runtime-abi linux-syscall-argv-key-value".to_string(),
    ];
    for artifact in agent_artifacts {
        if artifact.target_name != target_name {
            return Err(format!(
                "dependency artifact {} targets {}, expected {target_name}",
                artifact.file_name, artifact.target_name
            ));
        }
        lines.push(format!(
            "machine-bytecode-dependency {} {}",
            artifact.file_name,
            native_elf_dependency_identity(&artifact.bytes)?
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_compile_native_bytecode_report(
    action_name: &str,
    target_name: &str,
    target_executable: &[u8],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines =
        native_machine_bytecode_report_header("AIL-Compile-Native-Bytecode:", target_name)?;
    lines.push(format!("action {action_name}"));
    lines.push(format!(
        "machine-bytecode target {target_name} target.elf {} {} bytes {}",
        native_machine_bytecode_identity(target_executable)?,
        ail_artifact_fingerprint_bytes(target_executable),
        target_executable.len()
    ));
    for artifact in agent_artifacts {
        if artifact.target_name != target_name {
            return Err(format!(
                "native bytecode artifact {} targets {}, expected {target_name}",
                artifact.file_name, artifact.target_name
            ));
        }
        lines.push(format!(
            "machine-bytecode agent-target {} {} {} {} bytes {}",
            artifact.target_name,
            artifact.file_name,
            native_machine_bytecode_identity(&artifact.bytes)?,
            ail_artifact_fingerprint_bytes(&artifact.bytes),
            artifact.bytes.len()
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_compile_dependency_report(
    action_name: &str,
    target_name: &str,
    target_executable: &[u8],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Compile-Dependency-Report:".to_string(),
        format!("target {target_name}"),
        format!("action {action_name}"),
        "host-language-runtime none".to_string(),
        "dynamic-linker none".to_string(),
        "shared-libraries none".to_string(),
        "library-dependencies none".to_string(),
        "linker-invocation none".to_string(),
        "runtime-abi linux-syscall-argv-key-value".to_string(),
        format!(
            "machine-bytecode-dependency target.elf {}",
            native_elf_dependency_identity(target_executable)?
        ),
    ];
    for artifact in agent_artifacts {
        if artifact.target_name != target_name {
            return Err(format!(
                "dependency artifact {} targets {}, expected {target_name}",
                artifact.file_name, artifact.target_name
            ));
        }
        lines.push(format!(
            "machine-bytecode-dependency {} {}",
            artifact.file_name,
            native_elf_dependency_identity(&artifact.bytes)?
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn ail_compile_wasm_contract_status<'a>(
    program: &'a AilBytecodeProgram,
    target_name: &str,
) -> Result<&'a str, String> {
    if target_name != "wasm32-unknown-sandbox-wasm" {
        return Err(format!("unsupported Wasm contract target '{target_name}'"));
    }
    let status = program
        .target_support
        .get(target_name)
        .ok_or_else(|| {
            format!(
                "AIL-BACKEND-001 package {} target-support does not declare Wasm sandbox target {target_name}",
                program.package_name
            )
        })?
        .as_str();
    if matches!(status, "supported" | "supported-with-host-imports") {
        Ok(status)
    } else {
        Err(format!(
            "AIL-BACKEND-001 package {} target-support marks {target_name} as {status}; Wasm sandbox target {target_name} requires supported or supported-with-host-imports",
            program.package_name
        ))
    }
}

fn render_ail_compile_wasm_contract_report(
    program: &AilBytecodeProgram,
    action_name: &str,
    target_name: &str,
) -> Result<String, String> {
    let mut lines = ail_wasm_contract_report_header(program, target_name)?;
    lines.push(format!("action {action_name}"));
    lines.push("trace-scope reachable-action-call-graph".to_string());
    lines.push(format!(
        "trace-preservation {}",
        ail_wasm_contract_trace_preservation_label(program, action_name)?
    ));
    lines.push("executable-wasm-module none".to_string());
    push_ail_wasm_contract_host_import_lines(&mut lines, program);
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_compile_wasm_contract_bundle_report(
    program: &AilBytecodeProgram,
    target_name: &str,
) -> Result<String, String> {
    let mut lines = ail_wasm_contract_report_header(program, target_name)?;
    lines.push("bundle all-actions".to_string());
    lines.push("trace-scope reachable-action-call-graph".to_string());
    for action_name in program.actions.keys() {
        lines.push(format!(
            "action {action_name} trace-preservation {}",
            ail_wasm_contract_trace_preservation_label(program, action_name)?
        ));
    }
    lines.push("executable-wasm-module none".to_string());
    push_ail_wasm_contract_host_import_lines(&mut lines, program);
    Ok(format!("{}\n", lines.join("\n")))
}

fn ail_wasm_contract_report_header(
    program: &AilBytecodeProgram,
    target_name: &str,
) -> Result<Vec<String>, String> {
    let status = ail_compile_wasm_contract_status(program, target_name)?;
    Ok(vec![
        "AIL-Wasm-Contract-Report:".to_string(),
        format!(
            "package {} {}",
            program.package_name, program.package_version
        ),
        format!("target {target_name}"),
        format!("status {status}"),
        "bytecode-level portable-vm-contract".to_string(),
        "bytecode-container wasm-sandbox-contract".to_string(),
        "bytecode-format wasm32-contract-report".to_string(),
        format!(
            "host-boundary {}",
            if program.external_bindings_metadata_present {
                "declared-imports-only"
            } else {
                "saved-bytecode-contract"
            }
        ),
        format!(
            "host-import-metadata {}",
            if program.external_bindings_metadata_present {
                "present-in-saved-bytecode"
            } else {
                "not-present-in-saved-bytecode"
            }
        ),
    ])
}

fn ail_wasm_contract_trace_preservation_label(
    program: &AilBytecodeProgram,
    action_name: &str,
) -> Result<&'static str, String> {
    if ail_bytecode_action_reachable_trace_required(program, action_name)? {
        Ok("required")
    } else {
        Ok("not-required-by-action")
    }
}

fn push_ail_wasm_contract_host_import_lines(lines: &mut Vec<String>, program: &AilBytecodeProgram) {
    if program.external_bindings.is_empty() {
        lines.push(if program.external_bindings_metadata_present {
            "host-imports none".to_string()
        } else {
            "host-imports not-enumerated-in-saved-bytecode".to_string()
        });
    } else {
        for binding in program.external_bindings.values() {
            lines.push(format!(
                "host-import {} library {} symbol {} binding-kind {} calling-convention {}",
                binding.name,
                binding.library,
                binding.symbol,
                binding.binding_kind,
                binding.calling_convention
            ));
            for input in binding.inputs.values() {
                lines.push(format!(
                    "host-import-input {} {} {}",
                    binding.name,
                    input.name,
                    ail_wasm_contract_value_signature(input)
                ));
            }
            for output in binding.outputs.values() {
                lines.push(format!(
                    "host-import-output {} {} {}",
                    binding.name,
                    output.name,
                    ail_wasm_contract_value_signature(output)
                ));
            }
            for status_map in &binding.status_maps {
                lines.push(format!(
                    "host-import-status {} {} {}",
                    binding.name, status_map.code, status_map.target
                ));
            }
            for capability in &binding.capabilities {
                lines.push(format!(
                    "host-import-capability {} {}",
                    binding.name, capability
                ));
            }
            for trace in &binding.traces {
                lines.push(format!("host-import-trace {} {}", binding.name, trace));
            }
        }
    }
}

fn ail_bytecode_action_reachable_trace_required(
    program: &AilBytecodeProgram,
    action_name: &str,
) -> Result<bool, String> {
    let mut visited = BTreeSet::new();
    ail_bytecode_action_reachable_trace_required_inner(program, action_name, &mut visited)
}

fn ail_bytecode_action_reachable_trace_required_inner(
    program: &AilBytecodeProgram,
    action_name: &str,
    visited: &mut BTreeSet<String>,
) -> Result<bool, String> {
    if !visited.insert(action_name.to_string()) {
        return Ok(false);
    }
    let action = program
        .actions
        .get(action_name)
        .ok_or_else(|| format!("unknown AIL action '{action_name}'"))?;
    for instruction in &action.instructions {
        if instruction.opcode == "EMIT_TRACE" {
            return Ok(true);
        }
        if instruction.opcode == "CALL_ACTION" {
            let target = instruction.operands.get("target").ok_or_else(|| {
                format!("AIL bytecode action {action_name} CALL_ACTION is missing target")
            })?;
            if ail_bytecode_action_reachable_trace_required_inner(program, target, visited)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn render_ail_compile_wasm_contract_dependency_report(
    program: &AilBytecodeProgram,
    action_name: &str,
    target_name: &str,
) -> Result<String, String> {
    let _status = ail_compile_wasm_contract_status(program, target_name)?;
    if !program.actions.contains_key(action_name) {
        return Err(format!("unknown AIL action '{action_name}'"));
    }
    render_ail_compile_wasm_contract_dependency_report_for_scope(
        program,
        target_name,
        format!("action {action_name}"),
    )
}

fn render_ail_compile_wasm_contract_bundle_dependency_report(
    program: &AilBytecodeProgram,
    target_name: &str,
) -> Result<String, String> {
    let _status = ail_compile_wasm_contract_status(program, target_name)?;
    render_ail_compile_wasm_contract_dependency_report_for_scope(
        program,
        target_name,
        "bundle all-actions".to_string(),
    )
}

fn render_ail_compile_wasm_contract_dependency_report_for_scope(
    program: &AilBytecodeProgram,
    target_name: &str,
    scope_line: String,
) -> Result<String, String> {
    let library_dependencies = ail_bytecode_wasm_contract_library_dependencies(program);
    let lines = vec![
        "AIL-Compile-Dependency-Report:".to_string(),
        format!("target {target_name}"),
        scope_line,
        "host-language-runtime none".to_string(),
        "dynamic-linker none".to_string(),
        "shared-libraries none".to_string(),
        format!("library-dependencies {library_dependencies}"),
        "linker-invocation none".to_string(),
        "runtime-abi wasm32-declared-host-imports".to_string(),
        "machine-bytecode-dependency wasm-contract-report.txt portable-vm-contract".to_string(),
    ];
    let mut lines = lines;
    for binding in program.external_bindings.values() {
        lines.push(format!(
            "host-import-dependency {} library {} symbol {} binding-kind {} calling-convention {}",
            binding.name,
            binding.library,
            binding.symbol,
            binding.binding_kind,
            binding.calling_convention
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn ail_bytecode_wasm_contract_library_dependencies(program: &AilBytecodeProgram) -> String {
    if !program.external_bindings_metadata_present {
        return "not-enumerated-in-saved-bytecode".to_string();
    }
    let libraries = program
        .external_bindings
        .values()
        .map(|binding| binding.library.clone())
        .collect::<BTreeSet<_>>();
    if libraries.is_empty() {
        "none".to_string()
    } else {
        libraries.into_iter().collect::<Vec<_>>().join(",")
    }
}

fn ail_wasm_contract_value_signature(value: &ail::ail::AilExternalBindingValue) -> String {
    if value.ownership.is_empty() {
        value.type_name.clone()
    } else {
        format!("{} {}", value.type_name, value.ownership)
    }
}

fn render_ail_compile_bundle_native_bytecode_report(
    target_name: &str,
    target_executables: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines =
        native_machine_bytecode_report_header("AIL-Compile-Bundle-Native-Bytecode:", target_name)?;
    lines.push("bundle all-actions".to_string());
    for (role, artifacts) in [
        ("target", target_executables),
        ("agent-target", agent_artifacts),
    ] {
        for artifact in artifacts {
            if artifact.target_name != target_name {
                return Err(format!(
                    "native bytecode artifact {} targets {}, expected {target_name}",
                    artifact.file_name, artifact.target_name
                ));
            }
            lines.push(format!(
                "machine-bytecode {role} {} {} {} {} bytes {}",
                artifact.target_name,
                artifact.file_name,
                native_machine_bytecode_identity(&artifact.bytes)?,
                ail_artifact_fingerprint_bytes(&artifact.bytes),
                artifact.bytes.len()
            ));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_compile_bundle_dependency_report(
    target_name: &str,
    target_executables: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Compile-Bundle-Dependency-Report:".to_string(),
        format!("target {target_name}"),
        "bundle all-actions".to_string(),
        "host-language-runtime none".to_string(),
        "dynamic-linker none".to_string(),
        "shared-libraries none".to_string(),
        "library-dependencies none".to_string(),
        "linker-invocation none".to_string(),
        "runtime-abi linux-syscall-argv-key-value".to_string(),
    ];
    for artifacts in [target_executables, agent_artifacts] {
        for artifact in artifacts {
            if artifact.target_name != target_name {
                return Err(format!(
                    "dependency artifact {} targets {}, expected {target_name}",
                    artifact.file_name, artifact.target_name
                ));
            }
            lines.push(format!(
                "machine-bytecode-dependency {} {}",
                artifact.file_name,
                native_elf_dependency_identity(&artifact.bytes)?
            ));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_build_native_bytecode_report(
    target_name: &str,
    target_executable: &[u8],
    compiler_pass_artifacts: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines =
        native_machine_bytecode_report_header("AIL-Build-Native-Bytecode:", target_name)?;
    lines.push(format!(
        "machine-bytecode target {target_name} target.elf {} {} bytes {}",
        native_machine_bytecode_identity(target_executable)?,
        ail_artifact_fingerprint_bytes(target_executable),
        target_executable.len()
    ));
    for (role, artifacts) in [
        ("compiler-pass-target", compiler_pass_artifacts),
        ("agent-target", agent_artifacts),
    ] {
        for artifact in artifacts {
            if artifact.target_name != target_name {
                return Err(format!(
                    "native bytecode artifact {} targets {}, expected {target_name}",
                    artifact.file_name, artifact.target_name
                ));
            }
            lines.push(format!(
                "machine-bytecode {role} {} {} {} {} bytes {}",
                artifact.target_name,
                artifact.file_name,
                native_machine_bytecode_identity(&artifact.bytes)?,
                ail_artifact_fingerprint_bytes(&artifact.bytes),
                artifact.bytes.len()
            ));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_build_dependency_report(
    target_name: &str,
    target_executable: &[u8],
    compiler_pass_artifacts: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Build-Dependency-Report:".to_string(),
        format!("target {target_name}"),
        "host-language-runtime none".to_string(),
        "dynamic-linker none".to_string(),
        "shared-libraries none".to_string(),
        "library-dependencies none".to_string(),
        "linker-invocation none".to_string(),
        "runtime-abi linux-syscall-argv-key-value".to_string(),
        format!(
            "machine-bytecode-dependency target.elf {}",
            native_elf_dependency_identity(target_executable)?
        ),
    ];
    for artifacts in [compiler_pass_artifacts, agent_artifacts] {
        for artifact in artifacts {
            if artifact.target_name != target_name {
                return Err(format!(
                    "dependency artifact {} targets {}, expected {target_name}",
                    artifact.file_name, artifact.target_name
                ));
            }
            lines.push(format!(
                "machine-bytecode-dependency {} {}",
                artifact.file_name,
                native_elf_dependency_identity(&artifact.bytes)?
            ));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_pass_native_bytecode_report(
    target_name: &str,
    compiler_pass_artifacts: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines =
        native_machine_bytecode_report_header("AIL-Pass-Native-Bytecode:", target_name)?;
    for (role, artifacts) in [
        ("compiler-pass-target", compiler_pass_artifacts),
        ("agent-target", agent_artifacts),
    ] {
        for artifact in artifacts {
            if artifact.target_name != target_name {
                return Err(format!(
                    "native bytecode artifact {} targets {}, expected {target_name}",
                    artifact.file_name, artifact.target_name
                ));
            }
            lines.push(format!(
                "machine-bytecode {role} {} {} {} {} bytes {}",
                artifact.target_name,
                artifact.file_name,
                native_machine_bytecode_identity(&artifact.bytes)?,
                ail_artifact_fingerprint_bytes(&artifact.bytes),
                artifact.bytes.len()
            ));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_pass_dependency_report(
    target_name: &str,
    compiler_pass_artifacts: &[AilNativeArtifact],
    agent_artifacts: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Pass-Dependency-Report:".to_string(),
        format!("target {target_name}"),
        "host-language-runtime none".to_string(),
        "dynamic-linker none".to_string(),
        "shared-libraries none".to_string(),
        "library-dependencies none".to_string(),
        "linker-invocation none".to_string(),
        "runtime-abi linux-syscall-argv-key-value".to_string(),
    ];
    for artifacts in [compiler_pass_artifacts, agent_artifacts] {
        for artifact in artifacts {
            if artifact.target_name != target_name {
                return Err(format!(
                    "dependency artifact {} targets {}, expected {target_name}",
                    artifact.file_name, artifact.target_name
                ));
            }
            lines.push(format!(
                "machine-bytecode-dependency {} {}",
                artifact.file_name,
                native_elf_dependency_identity(&artifact.bytes)?
            ));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn native_machine_bytecode_identity(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.len() < 20 {
        return Err("native bytecode artifact is too small to contain an ELF header".to_string());
    }
    if &bytes[0..4] != b"\x7fELF" {
        return Err("native bytecode artifact is not an ELF executable".to_string());
    }
    let elf_class = bytes[4];
    let elf_data = bytes[5];
    let elf_type = u16::from_le_bytes([bytes[16], bytes[17]]);
    let elf_machine = u16::from_le_bytes([bytes[18], bytes[19]]);
    match (elf_class, elf_data, elf_type, elf_machine) {
        (2, 1, 2, 0x3e) => Ok("elf64-little-x86_64-executable"),
        _ => Err(format!(
            "unsupported native bytecode ELF identity class={elf_class} data={elf_data} type={elf_type} machine={elf_machine}"
        )),
    }
}

fn native_elf_dependency_identity(bytes: &[u8]) -> Result<&'static str, String> {
    native_machine_bytecode_identity(bytes)?;
    if bytes.len() < 64 {
        return Err("native bytecode artifact is too small to contain an ELF64 header".to_string());
    }
    let program_header_offset = u64::from_le_bytes([
        bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37], bytes[38], bytes[39],
    ]) as usize;
    let program_header_entry_size = u16::from_le_bytes([bytes[54], bytes[55]]) as usize;
    let program_header_count = u16::from_le_bytes([bytes[56], bytes[57]]) as usize;
    let headers_size = program_header_entry_size
        .checked_mul(program_header_count)
        .ok_or_else(|| "native ELF program header table size overflows".to_string())?;
    let headers_end = program_header_offset
        .checked_add(headers_size)
        .ok_or_else(|| "native ELF program header table offset overflows".to_string())?;
    if program_header_entry_size < 4 || headers_end > bytes.len() {
        return Err("native ELF program header table is outside the artifact".to_string());
    }
    for index in 0..program_header_count {
        let header_offset = program_header_offset + index * program_header_entry_size;
        let program_header_type = u32::from_le_bytes([
            bytes[header_offset],
            bytes[header_offset + 1],
            bytes[header_offset + 2],
            bytes[header_offset + 3],
        ]);
        match program_header_type {
            2 => return Err("native ELF declares a dynamic section dependency".to_string()),
            3 => return Err("native ELF declares a dynamic interpreter dependency".to_string()),
            _ => {}
        }
    }
    Ok("standalone-linux-syscall-elf")
}

fn draft_checked_ail_requirements_for_package(
    package: &ail::ail::AilPackage,
    prompt: &str,
    endpoint: &str,
    agent_requirements_context: Option<&str>,
) -> Result<(String, Vec<ail::ail::AilDiagnostic>), String> {
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

fn prompt_with_saved_interview_answers(
    prompt: &str,
    interview_file: Option<&str>,
) -> Result<String, String> {
    let Some(interview_file) = interview_file else {
        return Ok(prompt.to_string());
    };
    let interview_answers = fs::read_to_string(interview_file)
        .map_err(|error| format!("failed to read {interview_file}: {error}"))?;
    let interview_answers = interview_answers.trim();
    if interview_answers.is_empty() {
        return Err(format!("interview file {interview_file} is empty"));
    }
    Ok(format!(
        concat!("{}\n\n", "SAVED INTERVIEW ANSWERS:\n", "{}"),
        prompt, interview_answers
    ))
}

fn draft_checked_ail_spec_for_requirements(
    package: &ail::ail::AilPackage,
    prompt: &str,
    requirements: &str,
    endpoint: &str,
    agent_spec_context: Option<&str>,
) -> Result<ail::ail::AilDraftResult, String> {
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
    package: &ail::ail::AilPackage,
    requirements_file: &str,
) -> Result<(String, Vec<ail::ail::AilDiagnostic>), String> {
    let requirements = fs::read_to_string(requirements_file)
        .map_err(|error| format!("failed to read {requirements_file}: {error}"))?;
    let requirements = requirements.trim().to_string();
    let diagnostics = check_ail_requirements(package, &requirements);
    Ok((requirements, diagnostics))
}

fn parse_cli_ail_document(
    package: &ail::ail::AilPackage,
    cli_options: &CliOptions,
) -> Result<ail::ail::AilDocument, String> {
    if let Some(spec_file) = &cli_options.ail_spec_file {
        let spec_text = fs::read_to_string(spec_file)
            .map_err(|error| format!("failed to read {spec_file}: {error}"))?;
        return parse_ail_package_spec_text(package, &spec_text);
    }
    parse_ail_package_document(package)
}

fn parse_cli_ail_core(cli_options: &CliOptions) -> Result<ail::ail::AilCore, String> {
    let core_file = cli_options
        .ail_core_file
        .as_deref()
        .ok_or_else(|| "missing --core-file path".to_string())?;
    let core_text = fs::read_to_string(core_file)
        .map_err(|error| format!("failed to read {core_file}: {error}"))?;
    parse_ail_core_text(&core_text)
}

fn run_ail_core_action(
    core: &ail::ail::AilCore,
    document: &ail::ail::AilDocument,
    cli_options: &CliOptions,
) -> Result<u8, String> {
    let action = cli_options
        .ail_action
        .as_deref()
        .ok_or_else(|| "ail-run requires --action <name>".to_string())?;
    let bytecode = compile_ail_core_bytecode(core)?;
    let result = run_ail_bytecode_action(&bytecode, action, cli_options.runtime_state.clone())?;
    println!("ail-run {}", result.status);
    if let Some(failure) = &result.failure {
        println!("failure={failure}");
    }
    for line in render_ail_runtime_state_lines(document, &result.final_state) {
        println!("{line}");
    }
    if !result.trace.is_empty() {
        println!("trace={}", result.trace.join(" -> "));
    }
    Ok(if result.status == "succeeded" { 0 } else { 1 })
}

fn run_ail_compile_from_core(
    core: &ail::ail::AilCore,
    cli_options: &CliOptions,
    source_artifacts: Option<&AilSourcePackageArtifacts>,
) -> Result<u8, String> {
    let diagnostics = check_ail_core(core);
    if !diagnostics.is_empty() {
        for diagnostic in diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }
    if cli_options.ail_compile_all_actions {
        return run_ail_compile_bundle_from_core(core, cli_options, source_artifacts);
    }
    let action = cli_options
        .ail_action
        .as_deref()
        .ok_or_else(|| "ail-compile requires --action <name>".to_string())?;
    let target = cli_options
        .ail_compile_target
        .as_deref()
        .ok_or_else(|| "ail-compile requires --target <target>".to_string())?;
    if target == "wasm32-unknown-sandbox-wasm" {
        if cli_options.ail_compile_out.is_some() {
            return Err(
                "ail-compile wasm contract target does not emit --out yet; use --artifact-dir <dir>"
                    .to_string(),
            );
        }
        let artifact_dir = cli_options.artifact_dir.as_deref().ok_or_else(|| {
            "ail-compile wasm contract target requires --artifact-dir <dir>".to_string()
        })?;
        let bytecode = compile_ail_core_bytecode(core)?;
        let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
        let core_text = format!("{}\n", render_ail_core(core));
        let wasm_contract_report_text =
            render_ail_compile_wasm_contract_report(&bytecode, action, target)?;
        let dependency_report_text =
            render_ail_compile_wasm_contract_dependency_report(&bytecode, action, target)?;
        let agent_run = if let Some(agent_path) = &cli_options.ail_build_agent {
            let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
            let empty_agent_trace: &[String] = &[];
            let manifest_text =
                render_ail_compile_wasm_contract_manifest(&AilCompileWasmContractArtifactSet {
                    source_manifest_text: source_artifacts
                        .map(|artifacts| artifacts.manifest_text.as_str()),
                    source_spec_text: source_artifacts
                        .map(|artifacts| artifacts.spec_text.as_str()),
                    core_text: Some(&core_text),
                    bytecode_text: &bytecode_text,
                    scope: AilCompileWasmContractScope::Action(action),
                    target_name: target,
                    wasm_contract_report_text: &wasm_contract_report_text,
                    dependency_report_text: &dependency_report_text,
                    agent_bytecode_text: Some(agent_bytecode_text.as_str()),
                    agent_trace: Some(empty_agent_trace),
                });
            let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
            Some(run_ail_compile_wasm_contract_agent_verify_manifest(
                AilCompileWasmContractAgentManifestRequest {
                    agent_bytecode,
                    agent_bytecode_text,
                    package_name: &core.package.name,
                    bytecode_text: &bytecode_text,
                    source_artifacts,
                    wasm_contract_report_text: &wasm_contract_report_text,
                    dependency_report_text: &dependency_report_text,
                    manifest_text: &manifest_text,
                    manifest_fingerprint: &manifest_fingerprint,
                    target,
                },
            )?)
        } else {
            None
        };
        write_ail_compile_wasm_contract_artifacts(
            artifact_dir,
            AilCompileWasmContractArtifactSet {
                source_manifest_text: source_artifacts
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
                core_text: Some(&core_text),
                bytecode_text: &bytecode_text,
                scope: AilCompileWasmContractScope::Action(action),
                target_name: target,
                wasm_contract_report_text: &wasm_contract_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
                agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
            },
        )?;
        println!("ail-compile wrote {target} contract {artifact_dir}");
        return Ok(0);
    }
    let out = cli_options
        .ail_compile_out
        .as_deref()
        .ok_or_else(|| "ail-compile requires --out <path>".to_string())?;
    let executable = compile_ail_core_native_elf(core, action, target)?;
    write_native_executable(out, &executable)?;
    if let Some(artifact_dir) = &cli_options.artifact_dir {
        let bytecode = compile_ail_core_bytecode(core)?;
        let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
        let core_text = format!("{}\n", render_ail_core(core));
        let (
            agent_run,
            agent_native_artifacts,
            native_bytecode_report_text,
            dependency_report_text,
        ) = if let Some(agent_path) = &cli_options.ail_build_agent {
            let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
            let agent_native_artifacts =
                compile_ail_build_agent_native_artifacts(&agent_bytecode, target)?;
            let native_bytecode_report_text = render_ail_compile_native_bytecode_report(
                action,
                target,
                &executable,
                agent_native_artifacts.as_slice(),
            )?;
            let dependency_report_text = render_ail_compile_dependency_report(
                action,
                target,
                &executable,
                agent_native_artifacts.as_slice(),
            )?;
            let empty_agent_trace: &[String] = &[];
            let manifest_text = render_ail_compile_manifest(&AilCompileArtifactSet {
                source_manifest_text: source_artifacts
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
                core_text: Some(&core_text),
                bytecode_text: &bytecode_text,
                action_name: action,
                target_name: target,
                target_executable: &executable,
                native_bytecode_report_text: &native_bytecode_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: Some(agent_bytecode_text.as_str()),
                agent_trace: Some(empty_agent_trace),
                agent_native_executables: agent_native_artifacts.as_slice(),
            });
            let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
            let run = run_ail_compile_agent_verify_manifest(AilCompileAgentManifestRequest {
                agent_bytecode,
                agent_bytecode_text,
                package_name: &core.package.name,
                bytecode_text: &bytecode_text,
                source_artifacts,
                target_executable: &executable,
                native_bytecode_report_text: &native_bytecode_report_text,
                dependency_report_text: &dependency_report_text,
                manifest_text: &manifest_text,
                manifest_fingerprint: &manifest_fingerprint,
                target,
            })?;
            (
                Some(run),
                agent_native_artifacts,
                native_bytecode_report_text,
                dependency_report_text,
            )
        } else {
            let native_bytecode_report_text =
                render_ail_compile_native_bytecode_report(action, target, &executable, &[])?;
            let dependency_report_text =
                render_ail_compile_dependency_report(action, target, &executable, &[])?;
            (
                None,
                Vec::new(),
                native_bytecode_report_text,
                dependency_report_text,
            )
        };
        write_ail_compile_artifacts(
            artifact_dir,
            AilCompileArtifactSet {
                source_manifest_text: source_artifacts
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
                core_text: Some(&core_text),
                bytecode_text: &bytecode_text,
                action_name: action,
                target_name: target,
                target_executable: &executable,
                native_bytecode_report_text: &native_bytecode_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
                agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
                agent_native_executables: agent_native_artifacts.as_slice(),
            },
        )?;
    }
    println!("ail-compile wrote {target} executable {out}");
    Ok(0)
}

fn run_ail_compile_wasm_contract_bundle(
    bytecode: &AilBytecodeProgram,
    bytecode_text: &str,
    target: &str,
    artifact_dir: &str,
    core_text: Option<&str>,
    source_artifacts: Option<&AilSourcePackageArtifacts>,
    agent_path: Option<&str>,
) -> Result<u8, String> {
    let wasm_contract_report_text =
        render_ail_compile_wasm_contract_bundle_report(bytecode, target)?;
    let dependency_report_text =
        render_ail_compile_wasm_contract_bundle_dependency_report(bytecode, target)?;
    let agent_run = if let Some(agent_path) = agent_path {
        let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
        let empty_agent_trace: &[String] = &[];
        let manifest_text =
            render_ail_compile_wasm_contract_manifest(&AilCompileWasmContractArtifactSet {
                source_manifest_text: source_artifacts
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
                core_text,
                bytecode_text,
                scope: AilCompileWasmContractScope::AllActions,
                target_name: target,
                wasm_contract_report_text: &wasm_contract_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: Some(agent_bytecode_text.as_str()),
                agent_trace: Some(empty_agent_trace),
            });
        let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
        Some(run_ail_compile_wasm_contract_bundle_agent_verify_manifest(
            AilCompileWasmContractBundleAgentManifestRequest {
                agent_bytecode,
                agent_bytecode_text,
                package_name: &bytecode.package_name,
                bytecode_text,
                source_artifacts,
                wasm_contract_report_text: &wasm_contract_report_text,
                dependency_report_text: &dependency_report_text,
                manifest_text: &manifest_text,
                manifest_fingerprint: &manifest_fingerprint,
                target,
            },
        )?)
    } else {
        None
    };
    write_ail_compile_wasm_contract_artifacts(
        artifact_dir,
        AilCompileWasmContractArtifactSet {
            source_manifest_text: source_artifacts
                .map(|artifacts| artifacts.manifest_text.as_str()),
            source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
            core_text,
            bytecode_text,
            scope: AilCompileWasmContractScope::AllActions,
            target_name: target,
            wasm_contract_report_text: &wasm_contract_report_text,
            dependency_report_text: &dependency_report_text,
            agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
            agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
        },
    )?;
    println!("ail-compile wrote {target} contract bundle {artifact_dir}");
    Ok(0)
}

fn run_ail_compile_bundle_from_core(
    core: &ail::ail::AilCore,
    cli_options: &CliOptions,
    source_artifacts: Option<&AilSourcePackageArtifacts>,
) -> Result<u8, String> {
    let target = cli_options
        .ail_compile_target
        .as_deref()
        .ok_or_else(|| "ail-compile --all-actions requires --target <target>".to_string())?;
    let artifact_dir = cli_options
        .artifact_dir
        .as_deref()
        .ok_or_else(|| "ail-compile --all-actions requires --artifact-dir <dir>".to_string())?;
    let bytecode = compile_ail_core_bytecode(core)?;
    let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
    let core_text = format!("{}\n", render_ail_core(core));
    if target == "wasm32-unknown-sandbox-wasm" {
        return run_ail_compile_wasm_contract_bundle(
            &bytecode,
            &bytecode_text,
            target,
            artifact_dir,
            Some(&core_text),
            source_artifacts,
            cli_options.ail_build_agent.as_deref(),
        );
    }
    let target_executables = compile_ail_native_artifacts(&bytecode, target, "target")?;
    let (agent_run, agent_native_artifacts, native_bytecode_report_text, dependency_report_text) =
        if let Some(agent_path) = &cli_options.ail_build_agent {
            let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
            let agent_native_artifacts =
                compile_ail_build_agent_native_artifacts(&agent_bytecode, target)?;
            let native_bytecode_report_text = render_ail_compile_bundle_native_bytecode_report(
                target,
                target_executables.as_slice(),
                agent_native_artifacts.as_slice(),
            )?;
            let dependency_report_text = render_ail_compile_bundle_dependency_report(
                target,
                target_executables.as_slice(),
                agent_native_artifacts.as_slice(),
            )?;
            let empty_agent_trace: &[String] = &[];
            let manifest_text = render_ail_compile_bundle_manifest(&AilCompileBundleArtifactSet {
                source_manifest_text: source_artifacts
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
                core_text: Some(&core_text),
                bytecode_text: &bytecode_text,
                target_name: target,
                target_executables: target_executables.as_slice(),
                native_bytecode_report_text: &native_bytecode_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: Some(agent_bytecode_text.as_str()),
                agent_trace: Some(empty_agent_trace),
                agent_native_executables: agent_native_artifacts.as_slice(),
            });
            let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
            let run = run_ail_compile_bundle_agent_verify_manifest(
                AilCompileBundleAgentManifestRequest {
                    agent_bytecode,
                    agent_bytecode_text,
                    package_name: &core.package.name,
                    bytecode_text: &bytecode_text,
                    source_artifacts,
                    target,
                    target_executables: target_executables.as_slice(),
                    native_bytecode_report_text: &native_bytecode_report_text,
                    dependency_report_text: &dependency_report_text,
                    manifest_text: &manifest_text,
                    manifest_fingerprint: &manifest_fingerprint,
                },
            )?;
            (
                Some(run),
                agent_native_artifacts,
                native_bytecode_report_text,
                dependency_report_text,
            )
        } else {
            let native_bytecode_report_text = render_ail_compile_bundle_native_bytecode_report(
                target,
                target_executables.as_slice(),
                &[],
            )?;
            let dependency_report_text = render_ail_compile_bundle_dependency_report(
                target,
                target_executables.as_slice(),
                &[],
            )?;
            (
                None,
                Vec::new(),
                native_bytecode_report_text,
                dependency_report_text,
            )
        };
    write_ail_compile_bundle_artifacts(
        artifact_dir,
        AilCompileBundleArtifactSet {
            source_manifest_text: source_artifacts
                .map(|artifacts| artifacts.manifest_text.as_str()),
            source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
            core_text: Some(&core_text),
            bytecode_text: &bytecode_text,
            target_name: target,
            target_executables: target_executables.as_slice(),
            native_bytecode_report_text: &native_bytecode_report_text,
            dependency_report_text: &dependency_report_text,
            agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
            agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
            agent_native_executables: agent_native_artifacts.as_slice(),
        },
    )?;
    println!("ail-compile wrote {target} native bundle {artifact_dir}");
    Ok(0)
}

fn run_ail_compile_from_bytecode_file(path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    let target = cli_options
        .ail_compile_target
        .as_deref()
        .ok_or_else(|| "ail-compile requires --target <target>".to_string())?;
    let bytecode_text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let bytecode = parse_ail_bytecode(&bytecode_text)?;
    let diagnostics = verify_ail_bytecode(&bytecode);
    if !diagnostics.is_empty() {
        println!("ail-compile diagnostics:");
        for diagnostic in diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }
    if cli_options.ail_compile_all_actions {
        if target == "wasm32-unknown-sandbox-wasm" {
            let artifact_dir = cli_options.artifact_dir.as_deref().ok_or_else(|| {
                "ail-compile --all-actions requires --artifact-dir <dir>".to_string()
            })?;
            return run_ail_compile_wasm_contract_bundle(
                &bytecode,
                &bytecode_text,
                target,
                artifact_dir,
                None,
                None,
                cli_options.ail_build_agent.as_deref(),
            );
        }
        let artifact_dir = cli_options
            .artifact_dir
            .as_deref()
            .ok_or_else(|| "ail-compile --all-actions requires --artifact-dir <dir>".to_string())?;
        let target_executables = compile_ail_native_artifacts(&bytecode, target, "target")?;
        let (
            agent_run,
            agent_native_artifacts,
            native_bytecode_report_text,
            dependency_report_text,
        ) = if let Some(agent_path) = &cli_options.ail_build_agent {
            let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
            let agent_native_artifacts =
                compile_ail_build_agent_native_artifacts(&agent_bytecode, target)?;
            let native_bytecode_report_text = render_ail_compile_bundle_native_bytecode_report(
                target,
                target_executables.as_slice(),
                agent_native_artifacts.as_slice(),
            )?;
            let dependency_report_text = render_ail_compile_bundle_dependency_report(
                target,
                target_executables.as_slice(),
                agent_native_artifacts.as_slice(),
            )?;
            let empty_agent_trace: &[String] = &[];
            let manifest_text = render_ail_compile_bundle_manifest(&AilCompileBundleArtifactSet {
                source_manifest_text: None,
                source_spec_text: None,
                core_text: None,
                bytecode_text: &bytecode_text,
                target_name: target,
                target_executables: target_executables.as_slice(),
                native_bytecode_report_text: &native_bytecode_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: Some(agent_bytecode_text.as_str()),
                agent_trace: Some(empty_agent_trace),
                agent_native_executables: agent_native_artifacts.as_slice(),
            });
            let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
            let run = run_ail_compile_bundle_agent_verify_manifest(
                AilCompileBundleAgentManifestRequest {
                    agent_bytecode,
                    agent_bytecode_text,
                    package_name: &bytecode.package_name,
                    bytecode_text: &bytecode_text,
                    source_artifacts: None,
                    target,
                    target_executables: target_executables.as_slice(),
                    native_bytecode_report_text: &native_bytecode_report_text,
                    dependency_report_text: &dependency_report_text,
                    manifest_text: &manifest_text,
                    manifest_fingerprint: &manifest_fingerprint,
                },
            )?;
            (
                Some(run),
                agent_native_artifacts,
                native_bytecode_report_text,
                dependency_report_text,
            )
        } else {
            let native_bytecode_report_text = render_ail_compile_bundle_native_bytecode_report(
                target,
                target_executables.as_slice(),
                &[],
            )?;
            let dependency_report_text = render_ail_compile_bundle_dependency_report(
                target,
                target_executables.as_slice(),
                &[],
            )?;
            (
                None,
                Vec::new(),
                native_bytecode_report_text,
                dependency_report_text,
            )
        };
        write_ail_compile_bundle_artifacts(
            artifact_dir,
            AilCompileBundleArtifactSet {
                source_manifest_text: None,
                source_spec_text: None,
                core_text: None,
                bytecode_text: &bytecode_text,
                target_name: target,
                target_executables: target_executables.as_slice(),
                native_bytecode_report_text: &native_bytecode_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
                agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
                agent_native_executables: agent_native_artifacts.as_slice(),
            },
        )?;
        println!("ail-compile wrote {target} native bundle {artifact_dir}");
        return Ok(0);
    }
    let action = cli_options
        .ail_action
        .as_deref()
        .ok_or_else(|| "ail-compile requires --action <name>".to_string())?;
    if target == "wasm32-unknown-sandbox-wasm" {
        if cli_options.ail_compile_out.is_some() {
            return Err(
                "ail-compile wasm contract target does not emit --out yet; use --artifact-dir <dir>"
                    .to_string(),
            );
        }
        let artifact_dir = cli_options.artifact_dir.as_deref().ok_or_else(|| {
            "ail-compile wasm contract target requires --artifact-dir <dir>".to_string()
        })?;
        let wasm_contract_report_text =
            render_ail_compile_wasm_contract_report(&bytecode, action, target)?;
        let dependency_report_text =
            render_ail_compile_wasm_contract_dependency_report(&bytecode, action, target)?;
        let agent_run = if let Some(agent_path) = &cli_options.ail_build_agent {
            let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
            let empty_agent_trace: &[String] = &[];
            let manifest_text =
                render_ail_compile_wasm_contract_manifest(&AilCompileWasmContractArtifactSet {
                    source_manifest_text: None,
                    source_spec_text: None,
                    core_text: None,
                    bytecode_text: &bytecode_text,
                    scope: AilCompileWasmContractScope::Action(action),
                    target_name: target,
                    wasm_contract_report_text: &wasm_contract_report_text,
                    dependency_report_text: &dependency_report_text,
                    agent_bytecode_text: Some(agent_bytecode_text.as_str()),
                    agent_trace: Some(empty_agent_trace),
                });
            let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
            Some(run_ail_compile_wasm_contract_agent_verify_manifest(
                AilCompileWasmContractAgentManifestRequest {
                    agent_bytecode,
                    agent_bytecode_text,
                    package_name: &bytecode.package_name,
                    bytecode_text: &bytecode_text,
                    source_artifacts: None,
                    wasm_contract_report_text: &wasm_contract_report_text,
                    dependency_report_text: &dependency_report_text,
                    manifest_text: &manifest_text,
                    manifest_fingerprint: &manifest_fingerprint,
                    target,
                },
            )?)
        } else {
            None
        };
        write_ail_compile_wasm_contract_artifacts(
            artifact_dir,
            AilCompileWasmContractArtifactSet {
                source_manifest_text: None,
                source_spec_text: None,
                core_text: None,
                bytecode_text: &bytecode_text,
                scope: AilCompileWasmContractScope::Action(action),
                target_name: target,
                wasm_contract_report_text: &wasm_contract_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
                agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
            },
        )?;
        println!("ail-compile wrote {target} contract {artifact_dir}");
        return Ok(0);
    }
    let out = cli_options
        .ail_compile_out
        .as_deref()
        .ok_or_else(|| "ail-compile requires --out <path>".to_string())?;
    let executable = compile_ail_bytecode_native_elf(&bytecode, action, target)?;
    write_native_executable(out, &executable)?;
    if let Some(artifact_dir) = &cli_options.artifact_dir {
        let (
            agent_run,
            agent_native_artifacts,
            native_bytecode_report_text,
            dependency_report_text,
        ) = if let Some(agent_path) = &cli_options.ail_build_agent {
            let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
            let agent_native_artifacts =
                compile_ail_build_agent_native_artifacts(&agent_bytecode, target)?;
            let native_bytecode_report_text = render_ail_compile_native_bytecode_report(
                action,
                target,
                &executable,
                agent_native_artifacts.as_slice(),
            )?;
            let dependency_report_text = render_ail_compile_dependency_report(
                action,
                target,
                &executable,
                agent_native_artifacts.as_slice(),
            )?;
            let empty_agent_trace: &[String] = &[];
            let manifest_text = render_ail_compile_manifest(&AilCompileArtifactSet {
                source_manifest_text: None,
                source_spec_text: None,
                core_text: None,
                bytecode_text: &bytecode_text,
                action_name: action,
                target_name: target,
                target_executable: &executable,
                native_bytecode_report_text: &native_bytecode_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: Some(agent_bytecode_text.as_str()),
                agent_trace: Some(empty_agent_trace),
                agent_native_executables: agent_native_artifacts.as_slice(),
            });
            let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
            let run = run_ail_compile_agent_verify_manifest(AilCompileAgentManifestRequest {
                agent_bytecode,
                agent_bytecode_text,
                package_name: &bytecode.package_name,
                bytecode_text: &bytecode_text,
                source_artifacts: None,
                target_executable: &executable,
                native_bytecode_report_text: &native_bytecode_report_text,
                dependency_report_text: &dependency_report_text,
                manifest_text: &manifest_text,
                manifest_fingerprint: &manifest_fingerprint,
                target,
            })?;
            (
                Some(run),
                agent_native_artifacts,
                native_bytecode_report_text,
                dependency_report_text,
            )
        } else {
            let native_bytecode_report_text =
                render_ail_compile_native_bytecode_report(action, target, &executable, &[])?;
            let dependency_report_text =
                render_ail_compile_dependency_report(action, target, &executable, &[])?;
            (
                None,
                Vec::new(),
                native_bytecode_report_text,
                dependency_report_text,
            )
        };
        write_ail_compile_artifacts(
            artifact_dir,
            AilCompileArtifactSet {
                source_manifest_text: None,
                source_spec_text: None,
                core_text: None,
                bytecode_text: &bytecode_text,
                action_name: action,
                target_name: target,
                target_executable: &executable,
                native_bytecode_report_text: &native_bytecode_report_text,
                dependency_report_text: &dependency_report_text,
                agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
                agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
                agent_native_executables: agent_native_artifacts.as_slice(),
            },
        )?;
    }
    println!("ail-compile wrote {target} executable {out}");
    Ok(0)
}

fn run_ail_build_from_core(
    mut core: ail::ail::AilCore,
    cli_options: &CliOptions,
    source_artifacts: Option<AilSourcePackageArtifacts>,
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
    let mut pass_native_artifacts = Vec::new();
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
        if let Some(target) = &cli_options.ail_compile_target
            && cli_options.artifact_dir.is_some()
        {
            pass_native_artifacts = compile_ail_pass_native_artifacts(&pass_bytecode, target)?;
        }
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
            AilBuildPromptPortability {
                base_model: cli_options
                    .ail_build_base_model
                    .as_deref()
                    .or(cli_options.llm_endpoint.as_deref()),
                target_model: cli_options.ail_build_target_model.as_deref(),
            },
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
            let target_summary = format!("{target} executable {} bytes at {out}", executable.len());
            run_ail_build_agent_compile_native_target(
                agent_run,
                target,
                &target_summary,
                &target_fingerprint,
            )?;
            run_ail_build_agent_verify_target_artifact(
                agent_run,
                &target_summary,
                &target_fingerprint,
            )?;
        }
    }
    let prompt_portability_report = if let (Some(target_model), Some(agent_run)) = (
        cli_options.ail_build_target_model.as_deref(),
        agent_run.as_ref(),
    ) {
        Some(render_ail_prompt_portability_report(
            cli_options
                .ail_build_base_model
                .as_deref()
                .or(cli_options.llm_endpoint.as_deref())
                .unwrap_or(DEFAULT_BASE_LLM_ENDPOINT),
            target_model,
            requirements_artifact,
            agent_run,
        ))
    } else {
        None
    };
    if let Some(artifact_dir) = &cli_options.artifact_dir {
        let core_text = format!("{}\n", render_ail_core(&core));
        let flow_review_text = format!("{}\n", render_ail_flow_view(&core));
        let source_artifacts = source_artifacts.as_ref();
        let agent_native_artifacts = if let (Some((target, _, _)), Some(agent_run)) =
            (native_build.as_ref(), agent_run.as_ref())
        {
            compile_ail_build_agent_native_artifacts(&agent_run.bytecode, target)?
        } else {
            Vec::new()
        };
        let native_bytecode_report_text =
            if let Some((target, _, executable)) = native_build.as_ref() {
                Some(render_ail_build_native_bytecode_report(
                    target,
                    executable,
                    pass_native_artifacts.as_slice(),
                    agent_native_artifacts.as_slice(),
                )?)
            } else {
                None
            };
        let dependency_report_text = if let Some((target, _, executable)) = native_build.as_ref() {
            Some(render_ail_build_dependency_report(
                target,
                executable,
                pass_native_artifacts.as_slice(),
                agent_native_artifacts.as_slice(),
            )?)
        } else {
            None
        };
        if let Some(agent_run) = agent_run.as_mut() {
            let manifest_text = render_ail_build_manifest(&AilBuildArtifactSet {
                source_manifest_text: source_artifacts
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
                requirements: requirements_artifact,
                spec_text,
                core_text: &core_text,
                flow_review_text: &flow_review_text,
                bytecode_text: &bytecode_text,
                bytecode_fingerprint: &bytecode_fingerprint,
                prompt_portability_report: prompt_portability_report.as_deref(),
                target_name: native_build.as_ref().map(|(target, _, _)| target.as_str()),
                target_executable: native_build
                    .as_ref()
                    .map(|(_, _, executable)| executable.as_slice()),
                native_bytecode_report_text: native_bytecode_report_text.as_deref(),
                dependency_report_text: dependency_report_text.as_deref(),
                pass_bytecode_text: pass_bytecode_artifact.as_deref(),
                pass_bytecode_fingerprint: pass_bytecode_fingerprint_artifact.as_deref(),
                pass_trace: pass_trace_artifact.as_deref(),
                pass_native_executables: pass_native_artifacts.as_slice(),
                agent_bytecode_text: Some(agent_run.bytecode_text.as_str()),
                agent_trace: Some(agent_run.trace.as_slice()),
                agent_native_executables: agent_native_artifacts.as_slice(),
            });
            let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
            let source_package_text = source_artifacts.map(|artifacts| {
                ail_bootstrap_source_bundle_text(&artifacts.manifest_text, &artifacts.spec_text)
            });
            let source_package_fingerprint =
                source_package_text.as_deref().map(ail_artifact_fingerprint);
            let requirements_fingerprint = requirements_artifact.map(ail_artifact_fingerprint);
            let spec_fingerprint = spec_text.map(ail_artifact_fingerprint);
            let core_fingerprint = ail_artifact_fingerprint(&core_text);
            let flow_review_fingerprint = ail_artifact_fingerprint(&flow_review_text);
            let pass_target_fingerprint =
                native_artifact_fingerprint_text(pass_native_artifacts.as_slice());
            let prompt_portability_fingerprint = prompt_portability_report
                .as_deref()
                .map(ail_artifact_fingerprint);
            run_ail_build_agent_verify_manifest(
                agent_run,
                AilBuildAgentManifestVerification {
                    manifest_text: &manifest_text,
                    manifest_fingerprint: &manifest_fingerprint,
                    source_package_text: source_package_text.as_deref(),
                    source_package_fingerprint: source_package_fingerprint.as_deref(),
                    requirements_fingerprint: requirements_fingerprint.as_deref(),
                    spec_fingerprint: spec_fingerprint.as_deref(),
                    core_fingerprint: &core_fingerprint,
                    flow_review_fingerprint: &flow_review_fingerprint,
                    compiler_pass_target_fingerprint: pass_target_fingerprint.as_deref(),
                    prompt_portability_fingerprint: prompt_portability_fingerprint.as_deref(),
                    native_bytecode_report_text: native_bytecode_report_text.as_deref(),
                    dependency_report_text: dependency_report_text.as_deref(),
                },
            )?;
        }
        write_ail_build_artifacts(
            artifact_dir,
            AilBuildArtifactSet {
                source_manifest_text: source_artifacts
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
                requirements: requirements_artifact,
                spec_text,
                core_text: &core_text,
                flow_review_text: &flow_review_text,
                bytecode_text: &bytecode_text,
                bytecode_fingerprint: &bytecode_fingerprint,
                prompt_portability_report: prompt_portability_report.as_deref(),
                target_name: native_build.as_ref().map(|(target, _, _)| target.as_str()),
                target_executable: native_build
                    .as_ref()
                    .map(|(_, _, executable)| executable.as_slice()),
                native_bytecode_report_text: native_bytecode_report_text.as_deref(),
                dependency_report_text: dependency_report_text.as_deref(),
                pass_bytecode_text: pass_bytecode_artifact.as_deref(),
                pass_bytecode_fingerprint: pass_bytecode_fingerprint_artifact.as_deref(),
                pass_trace: pass_trace_artifact.as_deref(),
                pass_native_executables: pass_native_artifacts.as_slice(),
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

fn render_ail_bootstrap_source_conformance_report(
    path: &str,
    context: &str,
) -> Result<String, String> {
    if std::path::Path::new(path).is_file() {
        return Err(format!(
            "{context} requires an AIL package directory so conformance can run, found bytecode artifact {path}"
        ));
    }
    let package = load_ail_package_dir(path)?;
    let result = run_ail_conformance(&package)?;
    let report = render_ail_conformance_report(&result);
    if !result.success() {
        return Err(format!("{context} conformance failed:\n{report}"));
    }
    Ok(report)
}

fn load_ail_bootstrap_source_core(path: &str, context: &str) -> Result<ail::ail::AilCore, String> {
    if std::path::Path::new(path).is_file() {
        return Err(format!(
            "{context} requires an AIL package directory so checked core can be recorded, found bytecode artifact {path}"
        ));
    }
    let package = load_ail_package_dir(path)?;
    let document = parse_ail_package_document(&package)?;
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core(&core);
    if !diagnostics.is_empty() {
        return Err(format!(
            "{context} package has diagnostics before checked core recording:\n{}",
            diagnostics.join("\n")
        ));
    }
    Ok(core)
}

fn render_ail_bootstrap_source_core_artifact(path: &str, context: &str) -> Result<String, String> {
    let core = load_ail_bootstrap_source_core(path, context)?;
    Ok(format!("{}\n", render_ail_core(&core)))
}

fn load_ail_bootstrap_source_artifacts(
    path: &str,
    context: &str,
) -> Result<(String, String), String> {
    let artifacts = load_ail_source_package_artifacts(path, context)?;
    Ok((artifacts.manifest_text, artifacts.spec_text))
}

fn ensure_trailing_newline(text: String) -> String {
    if text.ends_with('\n') {
        text
    } else {
        format!("{text}\n")
    }
}

fn render_ail_bootstrap_fixed_point_report(
    first_pass_output_core_text: &str,
    second_pass_output_core_text: &str,
    second_pass_trace_text: &str,
) -> String {
    let changed = first_pass_output_core_text != second_pass_output_core_text;
    format!(
        "AIL-Bootstrap-Fixed-Point:\nfixed-point: {}\nfirst-pass-output {}\nsecond-pass-output {}\nsecond-pass-changed {}\nsecond-pass-trace {}\n",
        if changed { "changed" } else { "ok" },
        ail_artifact_fingerprint(first_pass_output_core_text),
        ail_artifact_fingerprint(second_pass_output_core_text),
        changed,
        ail_artifact_fingerprint(second_pass_trace_text)
    )
}

fn run_ail_bootstrap_command(path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    let target = cli_options
        .ail_compile_target
        .as_deref()
        .ok_or_else(|| "ail-bootstrap requires --target <target>".to_string())?;
    let artifact_dir = cli_options
        .artifact_dir
        .as_deref()
        .ok_or_else(|| "ail-bootstrap requires --artifact-dir <dir>".to_string())?;
    let pass_path = cli_options
        .ail_build_pass
        .as_deref()
        .ok_or_else(|| "ail-bootstrap requires --pass <compiler-pass>".to_string())?;
    let agent_path = cli_options
        .ail_build_agent
        .as_deref()
        .ok_or_else(|| "ail-bootstrap requires --agent <agent-package-or-bytecode>".to_string())?;

    let (toolchain_source_manifest_text, toolchain_source_spec_text) =
        load_ail_bootstrap_source_artifacts(path, "ail-bootstrap toolchain agent")?;
    let toolchain_core = load_ail_bootstrap_source_core(path, "ail-bootstrap toolchain agent")?;
    let toolchain_core_text = format!("{}\n", render_ail_core(&toolchain_core));
    let toolchain_conformance_report =
        render_ail_bootstrap_source_conformance_report(path, "ail-bootstrap toolchain agent")?;
    if toolchain_core.package.profile != "Application" {
        return Err(format!(
            "ail-bootstrap toolchain package must be Application profile, found {}",
            toolchain_core.package.profile
        ));
    }

    let (compiler_pass_bytecode, compiler_pass_bytecode_text) =
        load_ail_bytecode_or_compile_package(pass_path, "ail-bootstrap compiler pass")?;
    let (compiler_pass_source_manifest_text, compiler_pass_source_spec_text) =
        load_ail_bootstrap_source_artifacts(pass_path, "ail-bootstrap compiler pass")?;
    let compiler_pass_core_text =
        render_ail_bootstrap_source_core_artifact(pass_path, "ail-bootstrap compiler pass")?;
    let compiler_pass_conformance_report =
        render_ail_bootstrap_source_conformance_report(pass_path, "ail-bootstrap compiler pass")?;
    let compiler_pass_diagnostics = verify_ail_bytecode(&compiler_pass_bytecode);
    if !compiler_pass_diagnostics.is_empty() {
        return Err(format!(
            "ail-bootstrap compiler pass bytecode has diagnostics:\n{}",
            compiler_pass_diagnostics.join("\n")
        ));
    }
    if compiler_pass_bytecode.profile != "Compiler" {
        return Err(format!(
            "ail-bootstrap --pass requires a Compiler-profile bytecode artifact, found {}",
            compiler_pass_bytecode.profile
        ));
    }
    let compiler_pass_action = select_single_ail_pass_action(&compiler_pass_bytecode)?;
    let toolchain_pass_result = run_ail_compiler_pass_on_core(
        &compiler_pass_bytecode,
        &compiler_pass_action,
        &toolchain_core,
    )?;
    if toolchain_pass_result.run.status != "succeeded" {
        let mut message = format!("ail-bootstrap compiler pass {compiler_pass_action} failed");
        if let Some(failure) = toolchain_pass_result.run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !toolchain_pass_result.run.trace.is_empty() {
            message.push_str(&format!("\n{}", toolchain_pass_result.run.trace.join("\n")));
        }
        return Err(message);
    }
    let toolchain_pass_diagnostics = check_ail_core(&toolchain_pass_result.core);
    if !toolchain_pass_diagnostics.is_empty() {
        return Err(format!(
            "ail-bootstrap compiler pass output has diagnostics:\n{}",
            toolchain_pass_diagnostics.join("\n")
        ));
    }
    let toolchain_pass_output_core_text =
        format!("{}\n", render_ail_core(&toolchain_pass_result.core));
    let toolchain_pass_trace_text = format!("{}\n", toolchain_pass_result.run.trace.join("\n"));
    let fixed_point_pass_result = run_ail_compiler_pass_on_core(
        &compiler_pass_bytecode,
        &compiler_pass_action,
        &toolchain_pass_result.core,
    )?;
    if fixed_point_pass_result.run.status != "succeeded" {
        let mut message =
            format!("ail-bootstrap fixed-point compiler pass {compiler_pass_action} failed");
        if let Some(failure) = fixed_point_pass_result.run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !fixed_point_pass_result.run.trace.is_empty() {
            message.push_str(&format!(
                "\n{}",
                fixed_point_pass_result.run.trace.join("\n")
            ));
        }
        return Err(message);
    }
    let fixed_point_diagnostics = check_ail_core(&fixed_point_pass_result.core);
    if !fixed_point_diagnostics.is_empty() {
        return Err(format!(
            "ail-bootstrap fixed-point compiler pass output has diagnostics:\n{}",
            fixed_point_diagnostics.join("\n")
        ));
    }
    let fixed_point_output_core_text =
        format!("{}\n", render_ail_core(&fixed_point_pass_result.core));
    let fixed_point_trace_text = format!("{}\n", fixed_point_pass_result.run.trace.join("\n"));
    let fixed_point_report_text = render_ail_bootstrap_fixed_point_report(
        &toolchain_pass_output_core_text,
        &fixed_point_output_core_text,
        &fixed_point_trace_text,
    );
    if fixed_point_output_core_text != toolchain_pass_output_core_text {
        return Err(format!(
            "ail-bootstrap fixed-point changed compiler output:\n{fixed_point_report_text}"
        ));
    }
    let toolchain_bytecode = compile_ail_core_bytecode(&toolchain_pass_result.core)?;
    let toolchain_bytecode_text = format!("{}\n", render_ail_bytecode(&toolchain_bytecode));
    let toolchain_diagnostics = verify_ail_bytecode(&toolchain_bytecode);
    if !toolchain_diagnostics.is_empty() {
        return Err(format!(
            "ail-bootstrap toolchain bytecode has diagnostics:\n{}",
            toolchain_diagnostics.join("\n")
        ));
    }

    let toolchain_native_artifacts =
        compile_ail_native_artifacts(&toolchain_bytecode, target, "toolchain-agent")?;
    let compiler_pass_native_artifacts =
        compile_ail_native_artifacts(&compiler_pass_bytecode, target, "compiler-pass")?;
    let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
    let agent_native_artifacts = compile_ail_build_agent_native_artifacts(&agent_bytecode, target)?;
    let native_bytecode_report_text = render_ail_bootstrap_native_bytecode_report(
        target,
        toolchain_native_artifacts.as_slice(),
        compiler_pass_native_artifacts.as_slice(),
        agent_native_artifacts.as_slice(),
    )?;
    let host_boundary_report_text = render_ail_bootstrap_host_boundary_report(
        target,
        toolchain_native_artifacts.as_slice(),
        compiler_pass_native_artifacts.as_slice(),
        agent_native_artifacts.as_slice(),
    )?;
    let dependency_report_text = render_ail_bootstrap_dependency_report(
        target,
        toolchain_native_artifacts.as_slice(),
        compiler_pass_native_artifacts.as_slice(),
        agent_native_artifacts.as_slice(),
    )?;
    let handoff_report_text = render_ail_bootstrap_handoff_report(
        target,
        toolchain_native_artifacts.as_slice(),
        compiler_pass_native_artifacts.as_slice(),
        agent_native_artifacts.as_slice(),
    )?;
    let empty_agent_trace: &[String] = &[];
    let manifest_text = render_ail_bootstrap_manifest(&AilBootstrapArtifactSet {
        target_name: target,
        toolchain_source_manifest_text: &toolchain_source_manifest_text,
        toolchain_source_spec_text: &toolchain_source_spec_text,
        toolchain_core_text: &toolchain_core_text,
        toolchain_bytecode_text: &toolchain_bytecode_text,
        toolchain_conformance_report: &toolchain_conformance_report,
        toolchain_native_executables: toolchain_native_artifacts.as_slice(),
        compiler_pass_source_manifest_text: &compiler_pass_source_manifest_text,
        compiler_pass_source_spec_text: &compiler_pass_source_spec_text,
        compiler_pass_core_text: &compiler_pass_core_text,
        compiler_pass_bytecode_text: &compiler_pass_bytecode_text,
        toolchain_pass_output_core_text: &toolchain_pass_output_core_text,
        toolchain_pass_trace_text: &toolchain_pass_trace_text,
        fixed_point_report_text: &fixed_point_report_text,
        native_bytecode_report_text: &native_bytecode_report_text,
        host_boundary_report_text: &host_boundary_report_text,
        dependency_report_text: &dependency_report_text,
        handoff_report_text: &handoff_report_text,
        compiler_pass_conformance_report: &compiler_pass_conformance_report,
        compiler_pass_native_executables: compiler_pass_native_artifacts.as_slice(),
        agent_bytecode_text: Some(agent_bytecode_text.as_str()),
        agent_trace: Some(empty_agent_trace),
        agent_native_executables: agent_native_artifacts.as_slice(),
    });
    let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
    let agent_run = run_ail_bootstrap_agent_verify_manifest(AilBootstrapAgentManifestRequest {
        agent_bytecode,
        agent_bytecode_text,
        package_name: &toolchain_bytecode.package_name,
        toolchain_source_manifest_text: &toolchain_source_manifest_text,
        toolchain_source_spec_text: &toolchain_source_spec_text,
        toolchain_core_text: &toolchain_core_text,
        toolchain_bytecode_text: &toolchain_bytecode_text,
        compiler_pass_source_manifest_text: &compiler_pass_source_manifest_text,
        compiler_pass_source_spec_text: &compiler_pass_source_spec_text,
        compiler_pass_core_text: &compiler_pass_core_text,
        compiler_pass_bytecode_text: &compiler_pass_bytecode_text,
        toolchain_pass_output_core_text: &toolchain_pass_output_core_text,
        toolchain_pass_trace_text: &toolchain_pass_trace_text,
        fixed_point_report_text: &fixed_point_report_text,
        native_bytecode_report_text: &native_bytecode_report_text,
        host_boundary_report_text: &host_boundary_report_text,
        dependency_report_text: &dependency_report_text,
        handoff_report_text: &handoff_report_text,
        toolchain_conformance_report: &toolchain_conformance_report,
        compiler_pass_conformance_report: &compiler_pass_conformance_report,
        target_artifacts: toolchain_native_artifacts.as_slice(),
        compiler_pass_artifacts: compiler_pass_native_artifacts.as_slice(),
        manifest_text: &manifest_text,
        manifest_fingerprint: &manifest_fingerprint,
    })?;
    write_ail_bootstrap_artifacts(
        artifact_dir,
        AilBootstrapArtifactSet {
            target_name: target,
            toolchain_source_manifest_text: &toolchain_source_manifest_text,
            toolchain_source_spec_text: &toolchain_source_spec_text,
            toolchain_core_text: &toolchain_core_text,
            toolchain_bytecode_text: &toolchain_bytecode_text,
            toolchain_conformance_report: &toolchain_conformance_report,
            toolchain_native_executables: toolchain_native_artifacts.as_slice(),
            compiler_pass_source_manifest_text: &compiler_pass_source_manifest_text,
            compiler_pass_source_spec_text: &compiler_pass_source_spec_text,
            compiler_pass_core_text: &compiler_pass_core_text,
            compiler_pass_bytecode_text: &compiler_pass_bytecode_text,
            toolchain_pass_output_core_text: &toolchain_pass_output_core_text,
            toolchain_pass_trace_text: &toolchain_pass_trace_text,
            fixed_point_report_text: &fixed_point_report_text,
            native_bytecode_report_text: &native_bytecode_report_text,
            host_boundary_report_text: &host_boundary_report_text,
            dependency_report_text: &dependency_report_text,
            handoff_report_text: &handoff_report_text,
            compiler_pass_conformance_report: &compiler_pass_conformance_report,
            compiler_pass_native_executables: compiler_pass_native_artifacts.as_slice(),
            agent_bytecode_text: Some(agent_run.bytecode_text.as_str()),
            agent_trace: Some(agent_run.trace.as_slice()),
            agent_native_executables: agent_native_artifacts.as_slice(),
        },
    )?;
    println!("ail-bootstrap wrote {target} bootstrap bundle {artifact_dir}");
    Ok(0)
}

fn run_ail_command(command: &str, path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    if command == "ail-bootstrap" {
        return run_ail_bootstrap_command(path, cli_options);
    }
    if command == "ail-pass" {
        return run_ail_pass_command(path, cli_options);
    }
    if command == "ail-build" && cli_options.ail_core_file.is_some() {
        let core = parse_cli_ail_core(cli_options)?;
        return run_ail_build_from_core(core, cli_options, None, None, None, None, None);
    }
    if command == "ail-compile" && cli_options.ail_core_file.is_some() {
        let core = parse_cli_ail_core(cli_options)?;
        return run_ail_compile_from_core(&core, cli_options, None);
    }
    if command == "ail-run" && cli_options.ail_core_file.is_some() {
        let core = parse_cli_ail_core(cli_options)?;
        let diagnostics = check_ail_core(&core);
        if !diagnostics.is_empty() {
            for diagnostic in diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        let document = ail_document_from_core(&core);
        return run_ail_core_action(&core, &document, cli_options);
    }
    if command == "ail-compile" && std::path::Path::new(path).is_file() {
        return run_ail_compile_from_bytecode_file(path, cli_options);
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
        let core_text = format!("{}\n", render_ail_core(&core));
        let (
            agent_run,
            agent_native_artifacts,
            native_bytecode_report_text,
            dependency_report_text,
        ) = if let Some(agent_path) = &cli_options.ail_build_agent {
            let lower_agent = run_ail_lower_agent_verify_manifest(
                agent_path,
                &core,
                &core_text,
                &bytecode_text,
                None,
                cli_options.ail_compile_target.as_deref(),
            )?;
            (
                Some(lower_agent.agent_run),
                lower_agent.agent_native_artifacts,
                lower_agent.native_bytecode_report_text,
                lower_agent.dependency_report_text,
            )
        } else {
            (None, Vec::new(), None, None)
        };
        if let Some(artifact_dir) = &cli_options.artifact_dir {
            write_ail_lower_artifacts(
                artifact_dir,
                AilLowerArtifactSet {
                    source_manifest_text: None,
                    source_spec_text: None,
                    core_text: &core_text,
                    bytecode_text: &bytecode_text,
                    native_bytecode_report_text: native_bytecode_report_text.as_deref(),
                    dependency_report_text: dependency_report_text.as_deref(),
                    agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
                    agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
                    agent_native_executables: agent_native_artifacts.as_slice(),
                },
            )?;
        }
        print!("{bytecode_text}");
        return Ok(0);
    }
    if command == "ail-spec" && cli_options.ail_core_file.is_some() {
        let core = parse_cli_ail_core(cli_options)?;
        let diagnostics = check_ail_core(&core);
        if !diagnostics.is_empty() {
            println!("ail-spec core diagnostics:");
            for diagnostic in diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        println!("{}", render_ail_spec_from_core(&core));
        return Ok(0);
    }
    if command == "ail-patch" && cli_options.ail_core_file.is_some() {
        let core = parse_cli_ail_core(cli_options)?;
        let diagnostics = check_ail_core(&core);
        if !diagnostics.is_empty() {
            println!("ail-patch core diagnostics:");
            for diagnostic in diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        let Some(patch_path) = cli_options.patch_path.as_ref() else {
            return Err("ail-patch --core-file requires a patch file".to_string());
        };
        let patch_text = fs::read_to_string(patch_path)
            .map_err(|error| format!("failed to read {patch_path}: {error}"))?;
        let patched = apply_ail_core_patch_text(&core, &patch_text)?;
        let diagnostics = check_ail_core(&patched);
        if !diagnostics.is_empty() {
            println!("ail-patch diagnostics:");
            for diagnostic in diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        println!("{}", render_ail_core(&patched));
        return Ok(0);
    }
    if command == "ail-flow-edit" && cli_options.ail_core_file.is_some() {
        let core = parse_cli_ail_core(cli_options)?;
        let diagnostics = check_ail_core(&core);
        if !diagnostics.is_empty() {
            println!("ail-flow-edit core diagnostics:");
            for diagnostic in diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        let Some(edit_path) = cli_options.patch_path.as_ref() else {
            return Err("ail-flow-edit --core-file requires an edit file".to_string());
        };
        let edit_text = fs::read_to_string(edit_path)
            .map_err(|error| format!("failed to read {edit_path}: {error}"))?;
        let patched = apply_ail_flow_edit_text(&core, &edit_text)?;
        let diagnostics = check_ail_core(&patched);
        if !diagnostics.is_empty() {
            println!("ail-flow-edit diagnostics:");
            for diagnostic in diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        println!("{}", render_ail_core(&patched));
        return Ok(0);
    }
    if command == "ail-flow-edit" {
        return Err("ail-flow-edit requires --core-file <checked-core>".to_string());
    }
    let package = load_ail_package_dir(path)?;
    if command == "ail-conformance" {
        let result = run_ail_conformance(&package)?;
        let report_text = render_ail_conformance_report(&result);
        let mut agent_native_artifacts = Vec::new();
        let mut native_bytecode_report_text = None;
        let mut dependency_report_text = None;
        let agent_run = if let Some(agent_path) = &cli_options.ail_build_agent {
            let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
            if let Some(target) = &cli_options.ail_compile_target {
                agent_native_artifacts =
                    compile_ail_build_agent_native_artifacts(&agent_bytecode, target)?;
                native_bytecode_report_text = Some(render_ail_conformance_native_bytecode_report(
                    target,
                    agent_native_artifacts.as_slice(),
                )?);
                dependency_report_text = Some(render_ail_conformance_dependency_report(
                    target,
                    agent_native_artifacts.as_slice(),
                )?);
            }
            let empty_agent_trace: &[String] = &[];
            let manifest_text = render_ail_conformance_manifest(
                &result,
                &AilConformanceArtifactSet {
                    report_text: &report_text,
                    native_bytecode_report_text: native_bytecode_report_text.as_deref(),
                    dependency_report_text: dependency_report_text.as_deref(),
                    agent_bytecode_text: Some(agent_bytecode_text.as_str()),
                    agent_trace: Some(empty_agent_trace),
                    agent_native_executables: agent_native_artifacts.as_slice(),
                },
            );
            let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
            Some(run_ail_conformance_agent_verify_manifest(
                AilConformanceAgentManifestRequest {
                    agent_bytecode,
                    agent_bytecode_text,
                    package_name: &result.package_name,
                    report_text: &report_text,
                    manifest_text: &manifest_text,
                    manifest_fingerprint: &manifest_fingerprint,
                    native_bytecode_report_text: native_bytecode_report_text.as_deref(),
                    dependency_report_text: dependency_report_text.as_deref(),
                },
            )?)
        } else {
            None
        };
        if let Some(artifact_dir) = &cli_options.artifact_dir {
            write_ail_conformance_artifacts(
                artifact_dir,
                &result,
                AilConformanceArtifactSet {
                    report_text: &report_text,
                    native_bytecode_report_text: native_bytecode_report_text.as_deref(),
                    dependency_report_text: dependency_report_text.as_deref(),
                    agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
                    agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
                    agent_native_executables: agent_native_artifacts.as_slice(),
                },
            )?;
        }
        print!("{report_text}");
        if result.success() {
            return Ok(0);
        }
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
        if cli_options.diagnostics_json {
            print!(
                "{}",
                render_ail_draft_diagnostics_json(&result.spec_text, &result.diagnostics)
            );
            return if result.success() { Ok(0) } else { Ok(1) };
        }
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
        let prompt =
            prompt_with_saved_interview_answers(prompt, cli_options.ail_interview_file.as_deref())?;
        let endpoint = cli_options
            .llm_endpoint
            .as_deref()
            .unwrap_or(&package.metadata.base_llm_endpoint);
        let (requirements, diagnostics) =
            draft_checked_ail_requirements_for_package(&package, &prompt, endpoint, None)?;
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
    if command == "ail-interview" {
        let prompt = cli_options
            .ail_prompt
            .as_deref()
            .ok_or_else(|| "ail-interview requires --prompt <text>".to_string())?;
        let endpoint = cli_options
            .llm_endpoint
            .as_deref()
            .unwrap_or(&package.metadata.base_llm_endpoint);
        let interview = draft_ail_interview(&package, prompt, endpoint)?;
        let interview_text = format!("{interview}\n");
        if let Some(artifact_dir) = &cli_options.artifact_dir {
            write_ail_interview_artifacts(
                artifact_dir,
                AilInterviewArtifactSet {
                    package_name: &package.metadata.name,
                    package_version: &package.metadata.version,
                    interview_text: &interview_text,
                },
            )?;
        }
        print!("{interview_text}");
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
        let source_artifacts = load_ail_source_package_artifacts(path, "ail-build")?;
        let (requirements_artifact, spec_text, core, capture_prompt, agent_start) =
            if let Some(spec_file) = cli_options.ail_spec_file.as_deref() {
                let spec_text = fs::read_to_string(spec_file)
                    .map_err(|error| format!("failed to read {spec_file}: {error}"))?;
                let spec_text = spec_text.trim().to_string();
                let document = parse_ail_package_spec_text(&package, &spec_text)?;
                let core = elaborate_ail_core(&package, &document);
                let core_diagnostics = check_ail_core(&core);
                if !core_diagnostics.is_empty() {
                    println!("ail-build diagnostics:");
                    for diagnostic in core_diagnostics {
                        println!("{diagnostic}");
                    }
                    return Ok(1);
                }
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
                (None, spec_text, core, None, agent_start)
            } else {
                let prompt = cli_options
                    .ail_prompt
                    .as_deref()
                    .ok_or_else(|| "ail-build requires --prompt <text>".to_string())?;
                let requirements_prompt = prompt_with_saved_interview_answers(
                    prompt,
                    cli_options.ail_interview_file.as_deref(),
                )?;
                let mut agent_start = if let Some(agent_path) =
                    cli_options.ail_build_agent.as_deref()
                    && cli_options.ail_requirements_file.is_none()
                {
                    Some(run_ail_build_agent_capture(
                        agent_path,
                        &package.metadata.name,
                        &requirements_prompt,
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
                            &requirements_prompt,
                            endpoint,
                            agent_requirements_context.as_deref(),
                        )?
                    };
                let capture_prompt = cli_options
                    .ail_requirements_file
                    .is_none()
                    .then(|| requirements_prompt.clone());
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
            Some(source_artifacts),
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
            let core_text = format!("{}\n", render_ail_core(&core));
            let source_artifacts = load_ail_source_package_artifacts(path, "ail-lower")?;
            let (
                agent_run,
                agent_native_artifacts,
                native_bytecode_report_text,
                dependency_report_text,
            ) = if let Some(agent_path) = &cli_options.ail_build_agent {
                let lower_agent = run_ail_lower_agent_verify_manifest(
                    agent_path,
                    &core,
                    &core_text,
                    &bytecode_text,
                    Some(&source_artifacts),
                    cli_options.ail_compile_target.as_deref(),
                )?;
                (
                    Some(lower_agent.agent_run),
                    lower_agent.agent_native_artifacts,
                    lower_agent.native_bytecode_report_text,
                    lower_agent.dependency_report_text,
                )
            } else {
                (None, Vec::new(), None, None)
            };
            if let Some(artifact_dir) = &cli_options.artifact_dir {
                write_ail_lower_artifacts(
                    artifact_dir,
                    AilLowerArtifactSet {
                        source_manifest_text: Some(source_artifacts.manifest_text.as_str()),
                        source_spec_text: Some(source_artifacts.spec_text.as_str()),
                        core_text: &core_text,
                        bytecode_text: &bytecode_text,
                        native_bytecode_report_text: native_bytecode_report_text.as_deref(),
                        dependency_report_text: dependency_report_text.as_deref(),
                        agent_bytecode_text: agent_run
                            .as_ref()
                            .map(|run| run.bytecode_text.as_str()),
                        agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
                        agent_native_executables: agent_native_artifacts.as_slice(),
                    },
                )?;
            }
            print!("{bytecode_text}");
            Ok(0)
        }
        "ail-compile" => {
            let source_artifacts = load_ail_source_package_artifacts_with_spec_override(
                path,
                "ail-compile",
                cli_options.ail_spec_file.as_deref(),
            )?;
            run_ail_compile_from_core(&core, cli_options, Some(&source_artifacts))
        }
        "ail-run" => {
            if !diagnostics.is_empty() {
                for diagnostic in diagnostics {
                    println!("{diagnostic}");
                }
                return Ok(1);
            }
            run_ail_core_action(&core, &document, cli_options)
        }
        _ => Err(format!("unknown AIL command '{command}'")),
    }
}

fn parse_cli_options(command: &str, args: &[String]) -> Result<CliOptions, String> {
    let mut runtime_state = BTreeMap::new();
    let mut llm_endpoint = None;
    let mut artifact_dir = None;
    let mut patch_path = None;
    let mut ail_action = None;
    let mut ail_prompt = None;
    let mut ail_pass_target = None;
    let mut ail_build_pass = None;
    let mut ail_build_agent = None;
    let mut ail_build_base_model = None;
    let mut ail_build_target_model = None;
    let mut ail_interview_file = None;
    let mut ail_requirements_file = None;
    let mut ail_spec_file = None;
    let mut ail_core_file = None;
    let mut ail_compile_target = None;
    let mut ail_compile_out = None;
    let mut ail_compile_all_actions = false;
    let mut diagnostics_json = false;
    let mut index = 0;

    if command == "ail-patch" && args.get(index).is_none_or(|arg| arg != "--core-file") {
        let Some(path) = args.get(index) else {
            return Err("ail-patch requires a patch file".to_string());
        };
        patch_path = Some(path.clone());
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
                "ail-interview" | "ail-requirements" | "ail-spec" | "ail-draft" | "ail-build"
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
        if arg == "--interview-file" {
            if !matches!(command, "ail-requirements" | "ail-build") {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --interview-file".to_string());
            };
            ail_interview_file = Some(path.clone());
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
                "ail-lower"
                    | "ail-pass"
                    | "ail-compile"
                    | "ail-build"
                    | "ail-run"
                    | "ail-spec"
                    | "ail-patch"
                    | "ail-flow-edit"
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
        if matches!(command, "ail-patch" | "ail-flow-edit") {
            if patch_path.is_some() {
                return Err(usage());
            }
            patch_path = Some(arg.clone());
            index += 1;
            continue;
        }
        if arg == "--pass" {
            if !matches!(command, "ail-build" | "ail-bootstrap") {
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
            if !matches!(
                command,
                "ail-build"
                    | "ail-pass"
                    | "ail-lower"
                    | "ail-compile"
                    | "ail-conformance"
                    | "ail-bootstrap"
            ) {
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
            if !matches!(
                command,
                "ail-compile"
                    | "ail-build"
                    | "ail-pass"
                    | "ail-lower"
                    | "ail-conformance"
                    | "ail-bootstrap"
            ) {
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
        if arg == "--base-model" {
            if command != "ail-build" {
                return Err(usage());
            }
            let Some(model) = args.get(index + 1) else {
                return Err("missing value for --base-model".to_string());
            };
            ail_build_base_model = Some(model.clone());
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
        if arg == "--all-actions" {
            if command != "ail-compile" {
                return Err(usage());
            }
            ail_compile_all_actions = true;
            index += 1;
            continue;
        }
        if arg == "--llm-endpoint" {
            if !matches!(
                command,
                "ail-interview" | "ail-requirements" | "ail-spec" | "ail-draft" | "ail-build"
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
        if arg == "--diagnostics-json" {
            if command != "ail-draft" {
                return Err(usage());
            }
            diagnostics_json = true;
            index += 1;
            continue;
        }
        if arg == "--artifact-dir" {
            if !matches!(
                command,
                "ail-interview"
                    | "ail-build"
                    | "ail-pass"
                    | "ail-lower"
                    | "ail-compile"
                    | "ail-conformance"
                    | "ail-bootstrap"
            ) {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --artifact-dir".to_string());
            };
            artifact_dir = Some(path.clone());
            index += 2;
            continue;
        }
        if !matches!(command, "ail-run" | "ail-vm") {
            return Err(usage());
        }
        insert_runtime_state_arg(arg, &mut runtime_state)?;
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
    if command == "ail-spec" && ail_requirements_file.is_some() && ail_core_file.is_some() {
        return Err("--requirements-file cannot be combined with --core-file".to_string());
    }
    if command == "ail-build" && ail_build_target_model.is_some() && ail_build_agent.is_none() {
        return Err("--target-model requires --agent".to_string());
    }
    if command == "ail-build" && ail_build_base_model.is_some() && ail_build_target_model.is_none()
    {
        return Err("--base-model requires --target-model".to_string());
    }
    if command == "ail-compile" && ail_build_agent.is_some() && artifact_dir.is_none() {
        return Err("ail-compile --agent requires --artifact-dir <dir>".to_string());
    }
    if command == "ail-compile" && ail_compile_all_actions {
        if ail_compile_target.is_none() {
            return Err("ail-compile --all-actions requires --target <target>".to_string());
        }
        if artifact_dir.is_none() {
            return Err("ail-compile --all-actions requires --artifact-dir <dir>".to_string());
        }
        if ail_action.is_some() {
            return Err("ail-compile --all-actions cannot be combined with --action".to_string());
        }
        if ail_compile_out.is_some() {
            return Err("ail-compile --all-actions cannot be combined with --out".to_string());
        }
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
    if command == "ail-bootstrap" {
        if ail_build_pass.is_none() {
            return Err("ail-bootstrap requires --pass <compiler-pass>".to_string());
        }
        if ail_build_agent.is_none() {
            return Err("ail-bootstrap requires --agent <agent-package-or-bytecode>".to_string());
        }
        if ail_compile_target.is_none() {
            return Err("ail-bootstrap requires --target <target>".to_string());
        }
        if artifact_dir.is_none() {
            return Err("ail-bootstrap requires --artifact-dir <dir>".to_string());
        }
        if ail_action.is_some() {
            return Err("ail-bootstrap cannot be combined with --action".to_string());
        }
        if ail_compile_out.is_some() {
            return Err("ail-bootstrap cannot be combined with --out".to_string());
        }
    }
    if command == "ail-pass" && ail_core_file.is_none() && ail_pass_target.is_none() {
        return Err("ail-pass requires a target package or --core-file <path>".to_string());
    }
    if command == "ail-pass" && ail_compile_target.is_some() && artifact_dir.is_none() {
        return Err("ail-pass --target requires --artifact-dir <dir>".to_string());
    }
    if command == "ail-lower" && ail_build_agent.is_some() && artifact_dir.is_none() {
        return Err("ail-lower --agent requires --artifact-dir <dir>".to_string());
    }
    if command == "ail-lower" && ail_compile_target.is_some() && ail_build_agent.is_none() {
        return Err("ail-lower --target requires --agent <path>".to_string());
    }
    if command == "ail-lower" && ail_compile_target.is_some() && artifact_dir.is_none() {
        return Err("ail-lower --target requires --artifact-dir <dir>".to_string());
    }
    if command == "ail-conformance" && ail_build_agent.is_some() && artifact_dir.is_none() {
        return Err("ail-conformance --agent requires --artifact-dir <dir>".to_string());
    }
    if command == "ail-conformance" && ail_compile_target.is_some() && ail_build_agent.is_none() {
        return Err("ail-conformance --target requires --agent <path>".to_string());
    }
    if command == "ail-conformance" && ail_compile_target.is_some() && artifact_dir.is_none() {
        return Err("ail-conformance --target requires --artifact-dir <dir>".to_string());
    }

    Ok(CliOptions {
        runtime_state,
        llm_endpoint,
        artifact_dir,
        patch_path,
        ail_action,
        ail_prompt,
        ail_pass_target,
        ail_build_pass,
        ail_build_agent,
        ail_build_base_model,
        ail_build_target_model,
        ail_interview_file,
        ail_requirements_file,
        ail_spec_file,
        ail_core_file,
        ail_compile_target,
        ail_compile_out,
        ail_compile_all_actions,
        diagnostics_json,
    })
}

fn insert_runtime_state_arg(
    arg: &str,
    runtime_state: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let Some((key, value)) = arg.split_once('=') else {
        return Err(format!("runtime state '{arg}' must use key=value"));
    };
    if key.trim().is_empty() {
        return Err("runtime state key cannot be empty".to_string());
    }
    runtime_state.insert(key.to_string(), value.to_string());
    Ok(())
}
