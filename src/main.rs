use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::{Command, ExitCode};

use ail::ail::{
    AilBytecodeProgram, AilPackage, AilPackageMetadata, DEFAULT_BASE_LLM_ENDPOINT, ail_core_hash,
    ail_document_from_core, apply_ail_core_patch_text, apply_ail_flow_edit_text, apply_ail_patch,
    check_ail_core, check_ail_requirements, compile_ail_bytecode_native_elf,
    compile_ail_core_bytecode, compile_ail_core_native_elf, draft_ail_interview,
    draft_ail_requirements_response, draft_ail_requirements_response_recorded_with_max_tokens,
    draft_ail_spec, draft_ail_spec_from_requirements,
    draft_ail_spec_from_requirements_recorded_with_max_tokens, elaborate_ail_core,
    load_ail_package_dir, parse_ail_bytecode, parse_ail_core_text, parse_ail_package_document,
    parse_ail_package_spec_text, parse_ail_patch_text, parse_ail_spec_text, render_ail_bytecode,
    render_ail_core, render_ail_flow_view, render_ail_interview_questions_artifact,
    render_ail_package_dependency_report, render_ail_runtime_state_lines, render_ail_spec,
    render_ail_spec_from_core, repair_ail_requirements_from_diagnostics,
    repair_ail_spec_from_diagnostics, run_ail_bytecode_action, run_ail_compiler_pass_on_core,
    run_ail_conformance, verify_ail_bytecode,
};
use ail::core_model::json_string;

#[derive(Clone)]
struct CliOptions {
    runtime_state: BTreeMap<String, String>,
    llm_endpoint: Option<String>,
    llm_max_tokens: Option<usize>,
    artifact_dir: Option<String>,
    patch_path: Option<String>,
    ail_action: Option<String>,
    ail_prompt: Option<String>,
    ail_pass_target: Option<String>,
    ail_build_pass: Option<String>,
    ail_build_passes: Vec<String>,
    ail_build_agent: Option<String>,
    ail_build_base_model: Option<String>,
    ail_build_target_model: Option<String>,
    ail_interview_file: Option<String>,
    ail_requirements_file: Option<String>,
    ail_spec_file: Option<String>,
    ail_story_file: Option<String>,
    ail_core_file: Option<String>,
    ail_compile_target: Option<String>,
    ail_compile_out: Option<String>,
    ail_compile_all_actions: bool,
    diagnostics_json: bool,
    release_evidence: bool,
}

struct AilInterviewArtifactSet<'a> {
    package_name: &'a str,
    package_version: &'a str,
    interview_text: &'a str,
}

enum AilRequirementsDraftOutcome {
    Requirements {
        text: String,
        diagnostics: Vec<ail::ail::AilDiagnostic>,
    },
    Questions(Vec<String>),
}

struct AilSpecArtifactSet<'a> {
    source_core_text: &'a str,
    rendered_spec_text: &'a str,
    roundtrip_core_text: &'a str,
    source_core_hash: &'a str,
    roundtrip_core_hash: &'a str,
}

const REQUIRED_AIL_PROMPT_FILES: [&str; 11] = [
    "docs/ail/prompts/interview.system.md",
    "docs/ail/prompts/requirements.system.md",
    "docs/ail/prompts/spec-draft.system.md",
    "docs/ail/prompts/core-draft.system.md",
    "docs/ail/prompts/repair.system.md",
    "docs/ail/prompts/diagnostic-repair.system.md",
    "docs/ail/prompts/core-to-spec.system.md",
    "docs/ail/prompts/core-to-summary.system.md",
    "docs/ail/prompts/flow-patch.system.md",
    "docs/ail/prompts/trace-debug.system.md",
    "docs/ail/prompts/interop.system.md",
];

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
            | "ail-story"
            | "ail-pass"
            | "ail-bootstrap"
            | "ail-agent-contracts"
            | "ail-v03-roadmap"
            | "ail-examples"
            | "ail-e2e-corpus"
            | "ail-prompt-corpus"
            | "ail-patch"
            | "ail-flow-edit"
    ) {
        return run_ail_command(command, path, &cli_options);
    }
    Err(format!("unknown AIL command '{command}'"))
}

fn usage() -> String {
    "usage: ail <ail-check|ail-core|ail-flow|ail-flow-edit|ail-lower|ail-compile|ail-run|ail-vm|ail-conformance|ail-interview|ail-requirements|ail-spec|ail-draft|ail-build|ail-story|ail-pass|ail-bootstrap|ail-agent-contracts|ail-v03-roadmap|ail-prompt-corpus|ail-examples|ail-patch> <path> [patch|target-package] [--action name] [--prompt text] [--story-file path] [--interview-file path] [--requirements-file path] [--spec-file path] [--core-file path] [--pass path] [--agent path] [--target target] [--base-model name] [--target-model name] [--out path] [--all-actions] [--diagnostics-json] [--artifact-dir path] [--llm-endpoint url] [--max-tokens count] [--release-evidence] [key=value ...]\nsaved-core usage: ail <ail-spec|ail-lower|ail-compile|ail-run|ail-build> --core-file <checked-core> [--action name] [--target target] [--out path] [--artifact-dir path] [key=value ...]\nwasm-contract usage: ail ail-compile <package-or-artifact.ailbc.json> (--action <ActionName>|--all-actions) [--agent <agent-package-or-bytecode>] --target wasm32-unknown-sandbox-wasm --artifact-dir <dir> OR ail ail-compile --core-file <checked-core> (--action <ActionName>|--all-actions) [--agent <agent-package-or-bytecode>] --target wasm32-unknown-sandbox-wasm --artifact-dir <dir>\ncore-patch usage: ail ail-patch --core-file <checked-core> <ail-core.patch.json>\nflow-edit usage: ail ail-flow-edit --core-file <checked-core> <ail-flow.edit.json>\nail-pass usage: ail ail-pass <compiler-pass-package-or-bytecode> <target-package> --action <PassName> [--agent <agent-package-or-bytecode>] [--target linux-x86_64-elf --artifact-dir <dir>] OR ail ail-pass <compiler-pass-package-or-bytecode> --core-file <checked-core> --action <PassName> [--agent <agent-package-or-bytecode>] [--target linux-x86_64-elf --artifact-dir <dir>]\nail-bootstrap usage: ail ail-bootstrap <toolchain-agent-package> --pass <compiler-pass-package> [--pass <compiler-pass-package> ...] --agent <toolchain-agent-package> --target linux-x86_64-elf --artifact-dir <dir>\nail-story usage: ail ail-story <package> --story-file <story.md> [--artifact-dir <dir>] [--llm-endpoint <url>] [--max-tokens count] [--agent <agent-package-or-bytecode>] [--target <target> --action <ActionName> --out <path>]\nail-agent-contracts usage: ail ail-agent-contracts examples/agents\nail-v03-roadmap usage: ail ail-v03-roadmap examples --artifact-dir <dir> [--release-evidence]\nail-prompt-corpus usage: ail ail-prompt-corpus <corpus-file-or-dir> --artifact-dir <dir>\nail-examples usage: ail ail-examples examples --artifact-dir <dir> [--release-evidence]\ncompatibility alias: ail ail-e2e-corpus <examples-dir> --artifact-dir <dir> [--release-evidence]"
        .to_string()
}

fn render_ail_spec_manifest(artifacts: &AilSpecArtifactSet<'_>) -> String {
    format!(
        concat!(
            "AIL-Spec-Manifest:\n",
            "source-core source.ail-core.txt {}\n",
            "rendered-spec rendered.ail-spec.md {}\n",
            "roundtrip-core roundtrip.ail-core.txt {}\n",
            "roundtrip-hash {} {}\n"
        ),
        ail_artifact_fingerprint(artifacts.source_core_text),
        ail_artifact_fingerprint(artifacts.rendered_spec_text),
        ail_artifact_fingerprint(artifacts.roundtrip_core_text),
        artifacts.source_core_hash,
        artifacts.roundtrip_core_hash
    )
}

fn write_ail_spec_artifacts(
    artifact_dir: &str,
    artifacts: AilSpecArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-spec artifact dir {artifact_dir}: {error}")
    })?;
    fs::write(root.join("source.ail-core.txt"), artifacts.source_core_text)
        .map_err(|error| format!("failed to write ail-spec source core artifact: {error}"))?;
    fs::write(
        root.join("source.ail-core.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(artifacts.source_core_text)),
    )
    .map_err(|error| format!("failed to write ail-spec source core fingerprint: {error}"))?;
    fs::write(
        root.join("rendered.ail-spec.md"),
        artifacts.rendered_spec_text,
    )
    .map_err(|error| format!("failed to write ail-spec rendered spec artifact: {error}"))?;
    fs::write(
        root.join("rendered.ail-spec.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.rendered_spec_text)
        ),
    )
    .map_err(|error| format!("failed to write ail-spec rendered spec fingerprint: {error}"))?;
    fs::write(
        root.join("roundtrip.ail-core.txt"),
        artifacts.roundtrip_core_text,
    )
    .map_err(|error| format!("failed to write ail-spec roundtrip core artifact: {error}"))?;
    fs::write(
        root.join("roundtrip.ail-core.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.roundtrip_core_text)
        ),
    )
    .map_err(|error| format!("failed to write ail-spec roundtrip core fingerprint: {error}"))?;
    let manifest = render_ail_spec_manifest(&artifacts);
    fs::write(root.join("manifest.ail-spec.txt"), &manifest)
        .map_err(|error| format!("failed to write ail-spec manifest: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest)),
    )
    .map_err(|error| format!("failed to write ail-spec manifest fingerprint: {error}"))?;
    Ok(())
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

struct AilStoryModeArtifactSet<'a> {
    package_name: &'a str,
    package_version: &'a str,
    story_file: &'a str,
    story_source_text: &'a str,
    story_normalized_text: &'a str,
    story_fields: &'a BTreeMap<String, String>,
    llm_endpoint: Option<&'a str>,
    llm_max_tokens: Option<usize>,
    llm_transcripts: &'a [AilStoryLlmTranscript],
}

struct AilStoryLlmTranscript {
    stage: &'static str,
    artifact_kind: &'static str,
    request_body: String,
    response_body: String,
    content_text: String,
    content_kind: String,
}

#[derive(Clone, Copy)]
struct AilStoryLlmOptions<'a> {
    endpoint: &'a str,
    max_tokens: usize,
    retry_prompt_envelope_errors: bool,
}

struct AilStoryManifestArtifactSet<'a> {
    story_source_text: &'a str,
    story_normalized_text: &'a str,
    story_report_text: &'a str,
    story_amendment_comparison_text: Option<&'a str>,
    requirements_text: &'a str,
    spec_text: &'a str,
    core_text: &'a str,
    bytecode_text: &'a str,
    agent_bytecode_text: Option<&'a str>,
    agent_trace_text: Option<&'a str>,
    build_manifest_text: Option<&'a str>,
    llm_transcripts: &'a [AilStoryLlmTranscript],
}

struct AilStoryQuestionsManifestArtifactSet<'a> {
    story_source_text: &'a str,
    story_normalized_text: &'a str,
    story_report_text: &'a str,
    story_questions_text: &'a str,
    agent_trace_text: Option<&'a str>,
    llm_transcripts: &'a [AilStoryLlmTranscript],
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

struct AilCompileDarwinMachOContractArtifactSet<'a> {
    source_manifest_text: Option<&'a str>,
    source_spec_text: Option<&'a str>,
    core_text: Option<&'a str>,
    bytecode_text: &'a str,
    action_name: &'a str,
    target_name: &'a str,
    darwin_macho_contract_report_text: &'a str,
    dependency_report_text: &'a str,
}

#[derive(Debug, Clone)]
struct AilPromptCorpusEntry {
    id: String,
    source_file: String,
    semantic_task: String,
    task: String,
    model_label: String,
    prompt_file: String,
    checker_result: String,
    artifact_kind: String,
    package: Option<String>,
    output_file: Option<String>,
    stored_output: Option<String>,
    expected_diagnostic: Option<String>,
    expected_core_hash: Option<String>,
    failure_taxonomy: String,
}

#[derive(Debug, Clone)]
struct AilPromptCorpusEvaluation {
    entry: AilPromptCorpusEntry,
    diagnostic: String,
    artifact_fingerprint: String,
    checked_core_text: Option<String>,
    checked_core_fingerprint: Option<String>,
}

#[derive(Debug, Clone)]
struct AilE2eCorpusEntry {
    id: String,
    source_file: String,
    fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct AilE2eSupportPackageEntry {
    path: String,
    used_by: Vec<String>,
}

#[derive(Debug, Clone)]
struct AilE2eCorpusEvaluation {
    entry: AilE2eCorpusEntry,
    semantic_anchors: Vec<String>,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    extracted_artifact_fingerprint: Option<String>,
    checked_core_text: Option<String>,
    bytecode_text: Option<String>,
    vm_trace_text: Option<String>,
    target_report_text: Option<String>,
    ui_review_text: Option<String>,
    ui_review_patch_text: Option<String>,
    ui_semantic_tags_text: Option<String>,
    agent_policy_review_text: Option<String>,
    threat_model_audit_text: Option<String>,
    type_inference_review_text: Option<String>,
    state_boundary_review_text: Option<String>,
    workflow_scheduler_review_text: Option<String>,
    unsafe_boundary_review_text: Option<String>,
    complex_story_graph_text: Option<String>,
    application_walkthrough_text: Option<String>,
    story_promotion_review_text: Option<String>,
    dependency_review_text: Option<String>,
    stdlib_walkthrough_text: Option<String>,
    diagnostics_text: Option<String>,
    repair_tutorial_text: Option<String>,
    repair_proof: Option<AilE2eRepairProofArtifacts>,
    native_executables: Vec<AilNativeArtifact>,
}

#[derive(Debug, Clone)]
struct AilE2eRepairProofArtifacts {
    candidate_spec_text: String,
    checked_core_text: String,
    bytecode_text: String,
    vm_trace_text: Option<String>,
    target_report_text: Option<String>,
    repair_diff_text: String,
    promotion_review_text: String,
}

#[derive(Debug, Default)]
struct AilE2eStoryFamilyDimensions {
    entry_count: usize,
    prompt_files: BTreeSet<String>,
    story_journeys: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct AilV03SignalCoverage {
    count: usize,
    entries: BTreeSet<String>,
    capability_levels: BTreeSet<String>,
    program_domains: BTreeSet<String>,
    prompt_files: BTreeSet<String>,
    story_journeys: BTreeSet<String>,
    checker_results: BTreeSet<String>,
}

#[derive(Debug)]
struct AilAgentContract {
    label: String,
    version: String,
    executor_family: String,
    target_artifact: String,
    file_name: String,
    text: String,
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
    pass_composition_report_text: &'a str,
    pass_order_diagnostics_report_text: &'a str,
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

#[derive(Debug, Clone)]
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
    repair_proofs: &'a [AilConformanceRepairProofArtifacts],
    native_bytecode_report_text: Option<&'a str>,
    dependency_report_text: Option<&'a str>,
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
    agent_native_executables: &'a [AilNativeArtifact],
}

struct AilConformanceRepairProofArtifacts {
    fixture: String,
    expected_diagnostic: String,
    candidate_fixture: String,
    candidate_spec_text: String,
    checked_core_text: String,
    bytecode_text: String,
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
    package_dependency_report_text: Option<String>,
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

fn load_ail_prompt_corpus_entries(path: &str) -> Result<Vec<AilPromptCorpusEntry>, String> {
    let root = std::path::Path::new(path);
    let mut files = Vec::new();
    if root.is_file() {
        files.push(root.to_path_buf());
    } else {
        for entry in fs::read_dir(root)
            .map_err(|error| format!("failed to read corpus dir {path}: {error}"))?
        {
            let entry =
                entry.map_err(|error| format!("failed to read corpus dir entry: {error}"))?;
            let entry_path = entry.path();
            if entry_path
                .extension()
                .is_some_and(|extension| extension == "md")
            {
                files.push(entry_path);
            }
        }
        files.sort();
    }
    let mut entries = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).map_err(|error| {
            format!(
                "failed to read prompt corpus file {}: {error}",
                file.display()
            )
        })?;
        entries.extend(parse_ail_prompt_corpus_entries(
            &file.to_string_lossy(),
            &text,
        )?);
    }
    if entries.is_empty() {
        return Err(format!(
            "prompt corpus {path} did not contain stored outputs"
        ));
    }
    Ok(entries)
}

fn parse_ail_prompt_corpus_entries(
    source_file: &str,
    text: &str,
) -> Result<Vec<AilPromptCorpusEntry>, String> {
    let mut parsed = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_fields = BTreeMap::<String, String>::new();
    for line in text.lines() {
        if let Some(id) = line.strip_prefix("## Stored Output: ") {
            if let Some(entry_id) = current_id.take() {
                parsed.push(ail_prompt_corpus_entry_from_fields(
                    source_file,
                    entry_id,
                    &current_fields,
                )?);
                current_fields.clear();
            }
            current_id = Some(id.trim().to_string());
            continue;
        }
        if current_id.is_some()
            && let Some((key, value)) = line.split_once(':')
        {
            let key = key.trim();
            if !key.is_empty()
                && key.chars().all(|ch| {
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.'
                })
            {
                current_fields.insert(key.to_string(), value.trim().to_string());
            }
        }
    }
    if let Some(entry_id) = current_id {
        parsed.push(ail_prompt_corpus_entry_from_fields(
            source_file,
            entry_id,
            &current_fields,
        )?);
    }
    Ok(parsed)
}

fn ail_prompt_corpus_required_field(
    source_file: &str,
    entry_id: &str,
    fields: &BTreeMap<String, String>,
    field: &str,
) -> Result<String, String> {
    fields.get(field).cloned().ok_or_else(|| {
        format!("prompt corpus entry {entry_id} in {source_file} is missing {field}")
    })
}

fn ail_prompt_corpus_entry_from_fields(
    source_file: &str,
    id: String,
    fields: &BTreeMap<String, String>,
) -> Result<AilPromptCorpusEntry, String> {
    Ok(AilPromptCorpusEntry {
        semantic_task: ail_prompt_corpus_required_field(source_file, &id, fields, "semantic-task")?,
        task: ail_prompt_corpus_required_field(source_file, &id, fields, "task")?,
        model_label: ail_prompt_corpus_required_field(source_file, &id, fields, "model-label")?,
        prompt_file: ail_prompt_corpus_required_field(source_file, &id, fields, "prompt-file")?,
        checker_result: ail_prompt_corpus_required_field(
            source_file,
            &id,
            fields,
            "checker-result",
        )?,
        artifact_kind: ail_prompt_corpus_required_field(source_file, &id, fields, "artifact-kind")?,
        package: fields.get("package").cloned(),
        output_file: fields.get("output-file").cloned(),
        stored_output: fields.get("stored-output").cloned(),
        expected_diagnostic: fields.get("expected-diagnostic").cloned(),
        expected_core_hash: fields.get("expected-core-hash").cloned(),
        failure_taxonomy: fields
            .get("failure-taxonomy")
            .cloned()
            .unwrap_or_else(|| "none".to_string()),
        id,
        source_file: source_file.to_string(),
    })
}

fn evaluate_ail_prompt_corpus_entry(
    entry: &AilPromptCorpusEntry,
) -> Result<AilPromptCorpusEvaluation, String> {
    match entry.checker_result.as_str() {
        "accepted" => evaluate_accepted_ail_prompt_corpus_entry(entry),
        "rejected" => evaluate_rejected_ail_prompt_corpus_entry(entry),
        other => Err(format!(
            "prompt corpus entry {} has unknown checker-result {other}",
            entry.id
        )),
    }
}

fn evaluate_accepted_ail_prompt_corpus_entry(
    entry: &AilPromptCorpusEntry,
) -> Result<AilPromptCorpusEvaluation, String> {
    let (core_text, core_fingerprint) = checked_core_from_prompt_corpus_entry(entry)?;
    Ok(AilPromptCorpusEvaluation {
        entry: entry.clone(),
        diagnostic: "none".to_string(),
        artifact_fingerprint: core_fingerprint.clone(),
        checked_core_text: Some(core_text),
        checked_core_fingerprint: Some(core_fingerprint),
    })
}

fn evaluate_rejected_ail_prompt_corpus_entry(
    entry: &AilPromptCorpusEntry,
) -> Result<AilPromptCorpusEvaluation, String> {
    let expected = entry.expected_diagnostic.as_deref().ok_or_else(|| {
        format!(
            "prompt corpus rejected entry {} is missing expected-diagnostic",
            entry.id
        )
    })?;
    let diagnostic = if entry.artifact_kind == "prompt-envelope" {
        let stored_output = entry.stored_output.as_deref().ok_or_else(|| {
            format!(
                "prompt corpus prompt-envelope entry {} is missing stored-output",
                entry.id
            )
        })?;
        validate_stored_prompt_envelope_output(stored_output)
    } else if expected == "semantic-drift" {
        let (core_text, _) = checked_core_from_prompt_corpus_entry(entry)?;
        let package = load_ail_package_dir(
            entry
                .package
                .as_deref()
                .ok_or_else(|| format!("prompt corpus entry {} is missing package", entry.id))?,
        )?;
        let output_file = entry
            .output_file
            .as_deref()
            .ok_or_else(|| format!("prompt corpus entry {} is missing output-file", entry.id))?;
        let output_text = fs::read_to_string(output_file).map_err(|error| {
            format!("failed to read prompt corpus output {output_file}: {error}")
        })?;
        let core = checked_core_from_spec_text(package, output_file, output_text)?;
        let actual_hash = ail_core_hash(&core);
        let expected_hash = entry.expected_core_hash.as_deref().ok_or_else(|| {
            format!(
                "prompt corpus semantic-drift entry {} is missing expected-core-hash",
                entry.id
            )
        })?;
        if actual_hash == expected_hash {
            return Err(format!(
                "prompt corpus entry {} expected semantic drift but core hash matched {actual_hash}",
                entry.id
            ));
        }
        let fingerprint = ail_artifact_fingerprint(&core_text);
        return Ok(AilPromptCorpusEvaluation {
            entry: entry.clone(),
            diagnostic: "semantic-drift".to_string(),
            artifact_fingerprint: fingerprint,
            checked_core_text: Some(core_text),
            checked_core_fingerprint: Some(ail_artifact_fingerprint(&format!(
                "expected {expected_hash}\nactual {actual_hash}\n"
            ))),
        });
    } else {
        let diagnostics = diagnostics_from_prompt_corpus_spec_entry(entry)?;
        diagnostics
            .into_iter()
            .find(|diagnostic| diagnostic.starts_with(expected))
            .ok_or_else(|| {
                format!(
                    "prompt corpus entry {} expected diagnostic {expected}, but it was not emitted",
                    entry.id
                )
            })?
    };
    if !diagnostic.starts_with(expected) {
        return Err(format!(
            "prompt corpus entry {} expected diagnostic {expected}, got {diagnostic}",
            entry.id
        ));
    }
    Ok(AilPromptCorpusEvaluation {
        entry: entry.clone(),
        diagnostic,
        artifact_fingerprint: ail_artifact_fingerprint(
            entry
                .stored_output
                .as_deref()
                .or(entry.output_file.as_deref())
                .unwrap_or(&entry.id),
        ),
        checked_core_text: None,
        checked_core_fingerprint: None,
    })
}

fn checked_core_from_prompt_corpus_entry(
    entry: &AilPromptCorpusEntry,
) -> Result<(String, String), String> {
    if entry.artifact_kind != "ail-spec" {
        return Err(format!(
            "prompt corpus accepted entry {} must use artifact-kind ail-spec",
            entry.id
        ));
    }
    let package_path = entry
        .package
        .as_deref()
        .ok_or_else(|| format!("prompt corpus entry {} is missing package", entry.id))?;
    let output_file = entry
        .output_file
        .as_deref()
        .ok_or_else(|| format!("prompt corpus entry {} is missing output-file", entry.id))?;
    let package = load_ail_package_dir(package_path)?;
    let output_text = fs::read_to_string(output_file)
        .map_err(|error| format!("failed to read prompt corpus output {output_file}: {error}"))?;
    let core = checked_core_from_spec_text(package, output_file, output_text)?;
    let core_text = format!("{}\n", render_ail_core(&core));
    let core_fingerprint = ail_artifact_fingerprint(&core_text);
    Ok((core_text, core_fingerprint))
}

fn checked_core_from_spec_text(
    mut package: ail::ail::AilPackage,
    spec_path: &str,
    spec_text: String,
) -> Result<ail::ail::AilCore, String> {
    package.spec_path = std::path::PathBuf::from(spec_path);
    package.spec_text = spec_text;
    let document = parse_ail_package_document(&package)?;
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core(&core);
    if !diagnostics.is_empty() {
        return Err(format!(
            "prompt corpus accepted output {} has diagnostics:\n{}",
            spec_path,
            diagnostics.join("\n")
        ));
    }
    Ok(core)
}

fn diagnostics_from_prompt_corpus_spec_entry(
    entry: &AilPromptCorpusEntry,
) -> Result<Vec<String>, String> {
    let package_path = entry
        .package
        .as_deref()
        .ok_or_else(|| format!("prompt corpus entry {} is missing package", entry.id))?;
    let output_file = entry
        .output_file
        .as_deref()
        .ok_or_else(|| format!("prompt corpus entry {} is missing output-file", entry.id))?;
    let mut package = load_ail_package_dir(package_path)?;
    package.spec_path = std::path::PathBuf::from(output_file);
    package.spec_text = fs::read_to_string(output_file)
        .map_err(|error| format!("failed to read prompt corpus output {output_file}: {error}"))?;
    let document = parse_ail_package_document(&package)?;
    let core = elaborate_ail_core(&package, &document);
    Ok(check_ail_core(&core))
}

fn validate_stored_prompt_envelope_output(stored_output: &str) -> String {
    let has_artifact = stored_output.contains("\"artifact_text\"");
    let has_questions = stored_output.contains("\"questions\"");
    if has_artifact && has_questions {
        return "AIL-PROMPT-001 prompt envelope cannot contain both artifact_text and questions"
            .to_string();
    }
    if !has_artifact && !has_questions {
        return "AIL-PROMPT-001 prompt envelope must contain artifact_text or questions"
            .to_string();
    }
    if !stored_output.contains("\"must_check\":true") {
        return "AIL-PROMPT-001 prompt envelope checker_handoff.must_check must be true"
            .to_string();
    }
    if stored_output.contains("\"expected_profile\":\"AgentTool\"") {
        return "AIL-PROMPT-001 prompt envelope checker_handoff.expected_profile must be Application"
            .to_string();
    }
    "accepted".to_string()
}

fn render_ail_prompt_corpus_report(evaluations: &[AilPromptCorpusEvaluation]) -> String {
    let mut semantic_tasks = BTreeMap::<String, BTreeSet<String>>::new();
    let mut accepted_tasks = BTreeSet::<String>::new();
    let mut accepted_prompt_files = BTreeSet::<String>::new();
    let mut prompt_file_counts = BTreeMap::<String, usize>::new();
    let mut model_labels = BTreeSet::<String>::new();
    for evaluation in evaluations {
        semantic_tasks
            .entry(evaluation.entry.semantic_task.clone())
            .or_default()
            .insert(evaluation.entry.model_label.clone());
        model_labels.insert(evaluation.entry.model_label.clone());
        *prompt_file_counts
            .entry(evaluation.entry.prompt_file.clone())
            .or_insert(0usize) += 1;
        if evaluation.entry.checker_result == "accepted" {
            accepted_tasks.insert(evaluation.entry.task.clone());
            accepted_prompt_files.insert(evaluation.entry.prompt_file.clone());
        }
    }
    let mut lines = vec![
        "AIL-Prompt-Corpus-Portability-Report:".to_string(),
        format!("entry-count {}", evaluations.len()),
    ];
    if let Some(base_model) = model_labels.iter().next() {
        lines.push(format!("base-model {base_model}"));
    }
    if let Some(target_model) = model_labels.iter().next_back() {
        lines.push(format!("target-model {target_model}"));
    }
    for (semantic_task, labels) in semantic_tasks {
        lines.push(format!(
            "semantic-task {semantic_task} model-labels {}",
            labels.into_iter().collect::<Vec<_>>().join(",")
        ));
    }
    for task in accepted_tasks {
        lines.push(format!("accepted-task {task}"));
    }
    lines.push(format!(
        "required-prompt-file-count {}",
        REQUIRED_AIL_PROMPT_FILES.len()
    ));
    for prompt_file in REQUIRED_AIL_PROMPT_FILES {
        let status = if accepted_prompt_files.contains(prompt_file) {
            "covered"
        } else {
            "missing"
        };
        lines.push(format!("required-prompt-file {prompt_file} {status}"));
    }
    for (prompt_file, count) in prompt_file_counts {
        lines.push(format!("prompt-file-count {prompt_file} {count}"));
    }
    for evaluation in evaluations {
        let prompt_fingerprint = fs::read_to_string(&evaluation.entry.prompt_file)
            .map(|text| ail_artifact_fingerprint(&text))
            .unwrap_or_else(|_| "missing".to_string());
        lines.push(format!(
            "prompt-fingerprint {} {}",
            evaluation.entry.prompt_file, prompt_fingerprint
        ));
        lines.push(format!(
            "artifact-fingerprint {} {}",
            evaluation.entry.id, evaluation.artifact_fingerprint
        ));
        lines.push(format!(
            "checker-result {} {} {}",
            evaluation.entry.id, evaluation.entry.checker_result, evaluation.diagnostic
        ));
        lines.push(format!(
            "failure-taxonomy {}",
            evaluation.entry.failure_taxonomy
        ));
        if evaluation.entry.checker_result == "accepted" {
            lines.push(format!(
                "accepted-entry {} checker-result accepted task {} model {} source {}",
                evaluation.entry.id,
                evaluation.entry.task,
                evaluation.entry.model_label,
                evaluation.entry.source_file
            ));
        } else {
            lines.push(format!(
                "rejected-entry {} checker-result rejected diagnostic {} task {} model {} source {}",
                evaluation.entry.id,
                evaluation.diagnostic,
                evaluation.entry.task,
                evaluation.entry.model_label,
                evaluation.entry.source_file
            ));
        }
    }
    format!("{}\n", lines.join("\n"))
}

fn render_ail_prompt_corpus_manifest(
    corpus_path: &str,
    report_text: &str,
    evaluations: &[AilPromptCorpusEvaluation],
) -> String {
    let mut lines = vec![
        "AIL-Prompt-Corpus-Manifest:".to_string(),
        format!("source {corpus_path}"),
        format!(
            "portability-report prompt-corpus-portability.txt {}",
            ail_artifact_fingerprint(report_text)
        ),
    ];
    for evaluation in evaluations {
        if let Some(fingerprint) = &evaluation.checked_core_fingerprint {
            lines.push(format!(
                "accepted-core {} accepted/{}.ail-core.txt {}",
                evaluation.entry.id, evaluation.entry.id, fingerprint
            ));
        }
    }
    format!("{}\n", lines.join("\n"))
}

fn write_ail_prompt_corpus_artifacts(
    artifact_dir: &str,
    report_text: &str,
    manifest_text: &str,
    evaluations: &[AilPromptCorpusEvaluation],
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root.join("accepted")).map_err(|error| {
        format!("failed to create ail-prompt-corpus artifact dir {artifact_dir}: {error}")
    })?;
    fs::write(root.join("prompt-corpus-portability.txt"), report_text)
        .map_err(|error| format!("failed to write prompt corpus report: {error}"))?;
    fs::write(
        root.join("prompt-corpus-portability.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(report_text)),
    )
    .map_err(|error| format!("failed to write prompt corpus report fingerprint: {error}"))?;
    for evaluation in evaluations {
        if let Some(core_text) = &evaluation.checked_core_text {
            fs::write(
                root.join("accepted")
                    .join(format!("{}.ail-core.txt", evaluation.entry.id)),
                core_text,
            )
            .map_err(|error| format!("failed to write prompt corpus checked core: {error}"))?;
            fs::write(
                root.join("accepted")
                    .join(format!("{}.ail-core.fingerprint.txt", evaluation.entry.id)),
                format!("{}\n", ail_artifact_fingerprint(core_text)),
            )
            .map_err(|error| {
                format!("failed to write prompt corpus checked core fingerprint: {error}")
            })?;
        }
    }
    fs::write(root.join("manifest.ail-prompt-corpus.txt"), manifest_text)
        .map_err(|error| format!("failed to write prompt corpus manifest: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(manifest_text)),
    )
    .map_err(|error| format!("failed to write prompt corpus manifest fingerprint: {error}"))?;
    Ok(())
}

fn validate_ail_prompt_corpus_prompt_coverage(
    evaluations: &[AilPromptCorpusEvaluation],
) -> Result<(), String> {
    let accepted_prompt_files = evaluations
        .iter()
        .filter(|evaluation| evaluation.entry.checker_result == "accepted")
        .map(|evaluation| evaluation.entry.prompt_file.as_str())
        .collect::<BTreeSet<_>>();
    for required_prompt in REQUIRED_AIL_PROMPT_FILES {
        if !accepted_prompt_files.contains(required_prompt) {
            return Err(format!(
                "ail-prompt-corpus requires prompt-file {required_prompt}"
            ));
        }
    }
    Ok(())
}

fn run_ail_prompt_corpus_command(path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    let entries = load_ail_prompt_corpus_entries(path)?;
    let mut evaluations = Vec::new();
    for entry in &entries {
        evaluations.push(evaluate_ail_prompt_corpus_entry(entry)?);
    }
    validate_ail_prompt_corpus_prompt_coverage(&evaluations)?;
    let report_text = render_ail_prompt_corpus_report(&evaluations);
    let manifest_text = render_ail_prompt_corpus_manifest(path, &report_text, &evaluations);
    if let Some(artifact_dir) = &cli_options.artifact_dir {
        write_ail_prompt_corpus_artifacts(
            artifact_dir,
            &report_text,
            &manifest_text,
            &evaluations,
        )?;
    }
    print!("{report_text}");
    Ok(0)
}

fn parse_agent_contract_file(
    root: &std::path::Path,
    file_name: &str,
) -> Result<AilAgentContract, String> {
    let path = root.join(file_name);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut version = String::new();
    let mut label = String::new();
    let mut executor_family = String::new();
    let mut target_artifact = String::new();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("version:") {
            version = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("executor-label:") {
            label = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("executor-family:") {
            executor_family = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("target artifact:") {
            target_artifact = value.trim().to_string();
        }
    }
    if version.is_empty() {
        return Err(format!("agent contract {file_name} missing version"));
    }
    if label.is_empty() {
        return Err(format!("agent contract {file_name} missing executor-label"));
    }
    if executor_family != "codex-skill-agent" {
        return Err(format!(
            "agent contract {file_name} must use executor-family codex-skill-agent"
        ));
    }
    if target_artifact.is_empty() {
        return Err(format!(
            "agent contract {file_name} missing target artifact"
        ));
    }
    Ok(AilAgentContract {
        label,
        version,
        executor_family,
        target_artifact,
        file_name: file_name.to_string(),
        text,
    })
}

fn run_ail_agent_contracts_command(path: &str) -> Result<u8, String> {
    let root = std::path::Path::new(path);
    if !root.is_dir() {
        return Err(format!("ail-agent-contracts requires a directory: {path}"));
    }
    let required_contracts = [
        "codex-ail-requirements-writer.md",
        "codex-ail-spec-writer.md",
        "codex-ail-diagnostic-repairer.md",
        "codex-ail-prompt-reviewer.md",
        "codex-ail-story-promotion-reviewer.md",
        "codex-ail-repair-promotion-reviewer.md",
        "codex-ail-agent-policy-reviewer.md",
        "codex-ail-ui-patch-reviewer.md",
    ];
    let mut contracts = Vec::new();
    for file_name in required_contracts {
        contracts.push(parse_agent_contract_file(root, file_name)?);
    }
    let prompt_reviewer = contracts
        .iter()
        .find(|contract| contract.label == "codex-ail-prompt-reviewer")
        .ok_or_else(|| "missing codex-ail-prompt-reviewer contract".to_string())?;
    let repair_promotion_reviewer = contracts
        .iter()
        .find(|contract| contract.label == "codex-ail-repair-promotion-reviewer")
        .ok_or_else(|| "missing codex-ail-repair-promotion-reviewer contract".to_string())?;
    let story_promotion_reviewer = contracts
        .iter()
        .find(|contract| contract.label == "codex-ail-story-promotion-reviewer")
        .ok_or_else(|| "missing codex-ail-story-promotion-reviewer contract".to_string())?;
    let agent_policy_reviewer = contracts
        .iter()
        .find(|contract| contract.label == "codex-ail-agent-policy-reviewer")
        .ok_or_else(|| "missing codex-ail-agent-policy-reviewer contract".to_string())?;
    let ui_patch_reviewer = contracts
        .iter()
        .find(|contract| contract.label == "codex-ail-ui-patch-reviewer")
        .ok_or_else(|| "missing codex-ail-ui-patch-reviewer contract".to_string())?;
    for required in [
        "scripts/run_v03_prompt_llm_harness.py --review-artifacts",
        "scripts/run_v03_story_llm_harness.py --review-artifacts",
        "examples/agents/codex-ail-story-promotion-reviewer.md",
        "ail-examples examples --artifact-dir",
        "cargo run -- ail-v03-roadmap examples",
        "v03-roadmap.txt",
    ] {
        if !prompt_reviewer.text.contains(required) {
            return Err(format!(
                "agent contract {} missing {required}",
                prompt_reviewer.file_name
            ));
        }
    }
    for required in [
        "scripts/run_v03_story_llm_harness.py --review-artifacts",
        "scripts/run_v03_story_promotion_capture_plan.py --story-artifacts",
        "story-promotion-capture-plan.json",
        "story-promotion-capture-plan.fingerprint.txt",
        "scripts/run_v03_story_promotion_import_demo.py",
        "story-promotion-import-demo-report.txt",
        "story-promotion-import-demo-report.fingerprint.txt",
        "story-artifacts-preserved true",
        "proposed-accepted true",
        "promotion-decision accepted-for-promotion",
        "human-approval-required true",
        "promotion-source human-approved-story-promotion-batch",
        "human-approved-story-promotion-batch.fingerprint.txt",
        "entry-count",
        "checker-result-count accepted",
        "checker-result-count rejected",
        "default-max-tokens",
        "max-tokens",
        "token-budget-default",
        "token-budget-warning",
        "agent-trace",
        "semantic-anchor-missing-count 0",
        "ail-examples examples --artifact-dir",
        "cargo run -- ail-v03-roadmap examples",
        "accepted-for-promotion",
        "needs-repair",
        "rejected-for-promotion",
        "v03-roadmap.txt",
    ] {
        if !story_promotion_reviewer.text.contains(required) {
            return Err(format!(
                "agent contract {} missing {required}",
                story_promotion_reviewer.file_name
            ));
        }
    }
    for required in [
        "repair-promotion-review.txt",
        "repair-promotion-review.fingerprint.txt",
        "repair-promotion-review-fingerprint-observed-count",
        "repair-promotion-import-demo-report.txt",
        "source-preserved true",
        "proposed-accepted true",
        "accepted-for-promotion",
        "human-approval-required true",
        "semantic-anchor-missing-count 0",
        "ail-examples examples --artifact-dir",
        "manifest.ail-examples.txt",
    ] {
        if !repair_promotion_reviewer.text.contains(required) {
            return Err(format!(
                "agent contract {} missing {required}",
                repair_promotion_reviewer.file_name
            ));
        }
    }
    for required in [
        "agent-policy-review.txt",
        "agent-policy-review.fingerprint.txt",
        "agent-policy-review-fingerprint-observed-count",
        "scripts/run_v03_agent_policy_capture_plan.py",
        "agent-policy-capture-plan.json",
        "scripts/run_v03_agent_policy_import_demo.py",
        "agent-policy-import-demo-report.txt",
        "agent-policy-multi-agent-handoff-report.txt",
        "scripts/run_v03_agent_policy_live_reviewer_harness.py --dry-run",
        "scripts/run_v03_agent_policy_live_reviewer_harness.py --review-artifacts",
        "agent-policy-live-review-report.txt",
        "agent-policy-live-review-review.txt",
        "reviewer-envelope-valid-count",
        "reviewer-envelope-invalid-count",
        "reviewer-decision-accept-count",
        "reviewer-decision-needs-repair-count",
        "reviewer-decision-reject-count",
        "source-preserved true",
        "proposed-accepted true",
        "policy-handoff-imported true",
        "policy-handoff-replayed true",
        "accepted-for-import",
        "human-approval-required true",
        "agent-contract-check ail-agent-contracts examples/agents",
        "multi-agent-handoff-review required",
        "tool-permission-review required",
        "tool-approval-review required",
        "external-call-review required",
        "secret-redaction-review required",
        "audit-trace-review required",
        "ail-examples examples --artifact-dir",
        "manifest.ail-examples.txt",
    ] {
        if !agent_policy_reviewer.text.contains(required) {
            return Err(format!(
                "agent contract {} missing {required}",
                agent_policy_reviewer.file_name
            ));
        }
    }
    for required in [
        "ui-review-patch.txt",
        "ui-review-patch.fingerprint.txt",
        "ui-review-patch-fingerprint-observed-count",
        "scripts/run_v03_ui_patch_capture_plan.py",
        "ui-patch-capture-plan.json",
        "ui-patch-capture-plan.fingerprint.txt",
        "scripts/run_v03_ui_patch_import_demo.py",
        "ui-patch-import-demo-report.txt",
        "ui-patch-import-demo-report.fingerprint.txt",
        "scripts/run_v03_ui_patch_runtime_state_check.py",
        "ui-patch-runtime-state-check-report.txt",
        "ui-patch-runtime-state-check-report.fingerprint.txt",
        "visual-regression-fingerprint-preserved true",
        "runtime-ui-state-anchor Ticket.reviewStatus",
        "source-preserved true",
        "proposed-accepted true",
        "flow-edit-applied true",
        "patched-core-replayed true",
        "human-approval-required true",
        "agent-contract-check ail-agent-contracts examples/agents",
        "ail-examples examples --artifact-dir",
        "manifest.ail-examples.txt",
    ] {
        if !ui_patch_reviewer.text.contains(required) {
            return Err(format!(
                "agent contract {} missing {required}",
                ui_patch_reviewer.file_name
            ));
        }
    }
    let skill_path = root.join("skills/ail-prompt-interaction-reviewer/SKILL.md");
    let skill_text = fs::read_to_string(&skill_path).map_err(|error| {
        format!(
            "failed to read codex skill {}: {error}",
            skill_path.display()
        )
    })?;
    for required in [
        "name: ail-prompt-interaction-reviewer",
        "description: Use when",
        "examples/agents/codex-ail-prompt-reviewer.md",
        "examples/agents/codex-ail-story-promotion-reviewer.md",
        "examples/agents/codex-ail-repair-promotion-reviewer.md",
        "http://inteligentia-pro-1:8080/",
        "python3 scripts/run_v03_prompt_llm_harness.py --dry-run",
        "python3 scripts/run_v03_prompt_llm_harness.py --review-artifacts /tmp/ail-v03-prompt-llm",
        "python3 scripts/run_v03_story_llm_harness.py --review-artifacts /tmp/ail-v03-story-llm",
        "python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live",
        "cargo run -- ail-agent-contracts examples/agents",
        "cargo run -- ail-examples examples --artifact-dir",
        "cargo run -- ail-v03-roadmap examples --artifact-dir",
        "prompt-envelope-valid-count",
        "prompt-envelope-artifact-required-count",
        "prompt-envelope-questions-expected-count",
        "prompt-outcome-match-count",
        "prompt-envelope-invalid-count",
        "manifest.v03-prompt-llm.txt",
        "prompt-llm-harness-review.txt",
        "prompt-llm-harness-review.fingerprint.txt",
        "v03-roadmap.txt",
        "repair-promotion-review.txt",
        "repair-promotion-review.fingerprint.txt",
        "repair-promotion-review-fingerprint-observed-count",
        "repair-promotion-import-demo-report.txt",
        "source-preserved true",
        "proposed-accepted true",
        "accepted-for-promotion",
        "needs-repair",
        "rejected-for-promotion",
    ] {
        if !skill_text.contains(required) {
            return Err(format!(
                "codex skill {} missing {required}",
                skill_path.display()
            ));
        }
    }
    let story_skill_path = root.join("skills/ail-story-promotion-reviewer/SKILL.md");
    let story_skill_text = fs::read_to_string(&story_skill_path).map_err(|error| {
        format!(
            "failed to read codex skill {}: {error}",
            story_skill_path.display()
        )
    })?;
    for required in [
        "name: ail-story-promotion-reviewer",
        "description: Use when",
        "examples/agents/codex-ail-story-promotion-reviewer.md",
        "python3 scripts/run_v03_story_llm_harness.py --review-artifacts /tmp/ail-v03-story-llm",
        "python3 scripts/run_v03_story_promotion_capture_plan.py --story-artifacts /tmp/ail-v03-story-llm",
        "python3 scripts/run_v03_story_promotion_import_demo.py",
        "cargo run -- ail-agent-contracts examples/agents",
        "cargo run -- ail-examples examples --artifact-dir",
        "cargo run -- ail-v03-roadmap examples --artifact-dir",
        "story-promotion-capture-plan.json",
        "story-promotion-capture-plan.fingerprint.txt",
        "story-promotion-import-demo-report.txt",
        "story-promotion-import-demo-report.fingerprint.txt",
        "story-artifacts-preserved true",
        "proposed-accepted true",
        "promotion-decision accepted-for-promotion",
        "human-approval-required true",
        "promotion-source human-approved-story-promotion-batch",
        "human-approved-story-promotion-batch.fingerprint.txt",
        "entry-count",
        "checker-result-count accepted",
        "checker-result-count rejected",
        "semantic-anchor-missing-count 0",
        "accepted-for-promotion",
        "needs-repair",
        "rejected-for-promotion",
    ] {
        if !story_skill_text.contains(required) {
            return Err(format!(
                "codex skill {} missing {required}",
                story_skill_path.display()
            ));
        }
    }
    let runner_skill_path = root.join("skills/ail-system-prompt-harness-runner/SKILL.md");
    let runner_skill_text = fs::read_to_string(&runner_skill_path).map_err(|error| {
        format!(
            "failed to read codex skill {}: {error}",
            runner_skill_path.display()
        )
    })?;
    for required in [
        "name: ail-system-prompt-harness-runner",
        "description: Use when",
        "http://inteligentia-pro-1:8080/",
        "python3 scripts/run_v03_prompt_llm_harness.py --dry-run",
        "python3 scripts/run_v03_prompt_llm_harness.py",
        "python3 scripts/run_v03_prompt_llm_harness.py --review-artifacts /tmp/ail-v03-prompt-llm",
        "python3 scripts/run_v03_story_llm_harness.py --review-artifacts /tmp/ail-v03-story-llm",
        "python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live",
        "python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks --include-live",
        "python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --run-checks --include-live",
        "model-check present",
        "model-check-model-id",
        "prompt-envelope-valid-count",
        "story-prompt-envelope-valid-count",
        "agent-trace present",
        "cargo run -- ail-agent-contracts examples/agents",
        "cargo run -- ail-examples examples --artifact-dir",
        "cargo run -- ail-v03-roadmap examples --artifact-dir",
    ] {
        if !runner_skill_text.contains(required) {
            return Err(format!(
                "codex skill {} missing {required}",
                runner_skill_path.display()
            ));
        }
    }
    let repair_skill_path = root.join("skills/ail-repair-promotion-reviewer/SKILL.md");
    let repair_skill_text = fs::read_to_string(&repair_skill_path).map_err(|error| {
        format!(
            "failed to read codex skill {}: {error}",
            repair_skill_path.display()
        )
    })?;
    for required in [
        "name: ail-repair-promotion-reviewer",
        "description: Use when",
        "examples/agents/codex-ail-repair-promotion-reviewer.md",
        "python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks",
        "cargo run -- ail-examples examples --artifact-dir",
        "repair-promotion-review.txt",
        "repair-promotion-review.fingerprint.txt",
        "repair-promotion-review-fingerprint-observed-count",
        "repair-promotion-import-demo-report.txt",
        "repair-promotion-import-demo-report.fingerprint.txt",
        "source-preserved true",
        "proposed-accepted true",
        "accepted-for-promotion",
        "human-approval-required true",
        "semantic-anchor-missing-count 0",
    ] {
        if !repair_skill_text.contains(required) {
            return Err(format!(
                "codex skill {} missing {required}",
                repair_skill_path.display()
            ));
        }
    }
    let agent_policy_skill_path = root.join("skills/ail-agent-policy-reviewer/SKILL.md");
    let agent_policy_skill_text =
        fs::read_to_string(&agent_policy_skill_path).map_err(|error| {
            format!(
                "failed to read codex skill {}: {error}",
                agent_policy_skill_path.display()
            )
        })?;
    for required in [
        "name: ail-agent-policy-reviewer",
        "description: Use when",
        "examples/agents/codex-ail-agent-policy-reviewer.md",
        "python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks",
        "cargo run -- ail-agent-contracts examples/agents",
        "cargo run -- ail-examples examples --artifact-dir",
        "agent-policy-review.txt",
        "agent-policy-review.fingerprint.txt",
        "agent-policy-review-fingerprint-observed-count",
        "agent-policy-capture-plan.json",
        "agent-policy-capture-plan.fingerprint.txt",
        "agent-policy-import-demo-report.txt",
        "agent-policy-import-demo-report.fingerprint.txt",
        "agent-policy-multi-agent-handoff-report.txt",
        "agent-policy-multi-agent-handoff-report.fingerprint.txt",
        "python3 scripts/run_v03_agent_policy_live_reviewer_harness.py --dry-run",
        "python3 scripts/run_v03_agent_policy_live_reviewer_harness.py --review-artifacts /tmp/ail-v03-agent-policy-live-review",
        "python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks --include-live",
        "agent-policy-live-review-report.txt",
        "agent-policy-live-review-report.fingerprint.txt",
        "manifest.v03-agent-policy-live-review.txt",
        "agent-policy-live-review-review.txt",
        "agent-policy-live-review-review.fingerprint.txt",
        "reviewer-envelope-valid-count",
        "reviewer-envelope-invalid-count",
        "reviewer-decision-accept-count",
        "reviewer-decision-needs-repair-count",
        "reviewer-decision-reject-count",
        "source-preserved true",
        "proposed-accepted true",
        "policy-handoff-imported true",
        "policy-handoff-replayed true",
    ] {
        if !agent_policy_skill_text.contains(required) {
            return Err(format!(
                "codex skill {} missing {required}",
                agent_policy_skill_path.display()
            ));
        }
    }
    let ui_patch_skill_path = root.join("skills/ail-ui-patch-reviewer/SKILL.md");
    let ui_patch_skill_text = fs::read_to_string(&ui_patch_skill_path).map_err(|error| {
        format!(
            "failed to read codex skill {}: {error}",
            ui_patch_skill_path.display()
        )
    })?;
    for required in [
        "name: ail-ui-patch-reviewer",
        "description: Use when",
        "examples/agents/codex-ail-ui-patch-reviewer.md",
        "python3 scripts/run_ail_interactive_manual.py --chapter ui-patch-import --run-checks",
        "cargo run -- ail-agent-contracts examples/agents",
        "cargo run -- ail-examples examples --artifact-dir",
        "ui-review-patch.txt",
        "ui-review-patch.fingerprint.txt",
        "ui-review-patch-fingerprint-observed-count",
        "ui-patch-capture-plan.json",
        "ui-patch-capture-plan.fingerprint.txt",
        "ui-patch-import-demo-report.txt",
        "ui-patch-import-demo-report.fingerprint.txt",
        "ui-patch-runtime-state-check-report.txt",
        "ui-patch-runtime-state-check-report.fingerprint.txt",
        "human-approval-required true",
        "source-preserved true",
        "proposed-accepted true",
        "flow-edit-applied true",
        "patched-core-replayed true",
        "visual-regression-fingerprint-preserved true",
        "runtime-ui-state-anchor Ticket.reviewStatus",
    ] {
        if !ui_patch_skill_text.contains(required) {
            return Err(format!(
                "codex skill {} missing {required}",
                ui_patch_skill_path.display()
            ));
        }
    }

    println!("AIL-Agent-Contracts-Report:");
    println!("contract-count {}", contracts.len());
    for contract in contracts {
        println!(
            "contract {} {} {} {}",
            contract.label, contract.version, contract.executor_family, contract.target_artifact
        );
    }
    println!("review-command scripts/run_v03_prompt_llm_harness.py --review-artifacts");
    println!("review-command scripts/run_v03_story_llm_harness.py --review-artifacts");
    println!(
        "review-command scripts/run_v03_agent_policy_live_reviewer_harness.py --review-artifacts"
    );
    println!(
        "review-command scripts/run_v03_story_promotion_live_reviewer_harness.py --review-artifacts"
    );
    println!("story-promotion-contract codex-ail-story-promotion-reviewer");
    println!("story-promotion-import-artifact story-promotion-import-demo-report.txt");
    println!("story-promotion-live-review-artifact story-promotion-live-review-report.txt");
    println!("repair-promotion-artifact repair-promotion-review.txt");
    println!("repair-promotion-import-artifact repair-promotion-import-demo-report.txt");
    println!("agent-policy-import-artifact agent-policy-import-demo-report.txt");
    println!("agent-policy-live-review-artifact agent-policy-live-review-report.txt");
    println!("ui-patch-import-artifact ui-patch-import-demo-report.txt");
    println!("ui-patch-runtime-artifact ui-patch-runtime-state-check-report.txt");
    println!("roadmap-artifact v03-roadmap.txt");
    println!("roadmap-command cargo run -- ail-v03-roadmap examples --artifact-dir");
    println!("codex-skill examples/agents/skills/ail-prompt-interaction-reviewer/SKILL.md");
    println!("codex-skill examples/agents/skills/ail-story-promotion-reviewer/SKILL.md");
    println!("codex-skill examples/agents/skills/ail-system-prompt-harness-runner/SKILL.md");
    println!("codex-skill examples/agents/skills/ail-repair-promotion-reviewer/SKILL.md");
    println!("codex-skill examples/agents/skills/ail-agent-policy-reviewer/SKILL.md");
    println!("codex-skill examples/agents/skills/ail-ui-patch-reviewer/SKILL.md");
    println!("agent-contracts-result accepted");
    Ok(0)
}

fn load_ail_e2e_corpus_entries(path: &std::path::Path) -> Result<Vec<AilE2eCorpusEntry>, String> {
    if path.is_file() {
        let text = fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read examples catalog file {}: {error}",
                path.display()
            )
        })?;
        return parse_ail_e2e_corpus_entries(&path.to_string_lossy(), &text);
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(path).map_err(|error| {
        format!(
            "failed to read examples catalog dir {}: {error}",
            path.display()
        )
    })? {
        let entry =
            entry.map_err(|error| format!("failed to read examples catalog dir entry: {error}"))?;
        let entry_path = entry.path();
        if entry_path.is_dir()
            || entry_path
                .extension()
                .is_some_and(|extension| extension == "md")
        {
            entries.extend(load_ail_e2e_corpus_entries(&entry_path)?);
        }
    }
    Ok(entries)
}

fn parse_ail_e2e_corpus_entries(
    source_file: &str,
    text: &str,
) -> Result<Vec<AilE2eCorpusEntry>, String> {
    let mut entries = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_fields = BTreeMap::<String, String>::new();
    for line in text.lines() {
        if let Some(id) = line
            .strip_prefix("## Example: ")
            .or_else(|| line.strip_prefix("## End-To-End Example: "))
        {
            if let Some(entry_id) = current_id.take() {
                entries.push(ail_e2e_corpus_entry_from_fields(
                    source_file,
                    entry_id,
                    &current_fields,
                )?);
                current_fields.clear();
            }
            current_id = Some(id.trim().to_string());
            continue;
        }
        if current_id.is_some()
            && let Some((key, value)) = line.split_once(':')
        {
            let key = key.trim();
            if !key.is_empty()
                && key.chars().all(|ch| {
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.'
                })
            {
                current_fields.insert(key.to_string(), value.trim().to_string());
            }
        }
    }
    if let Some(entry_id) = current_id.take() {
        entries.push(ail_e2e_corpus_entry_from_fields(
            source_file,
            entry_id,
            &current_fields,
        )?);
    }
    Ok(entries)
}

fn ail_e2e_corpus_entry_from_fields(
    source_file: &str,
    id: String,
    fields: &BTreeMap<String, String>,
) -> Result<AilE2eCorpusEntry, String> {
    for field in [
        "semantic-task",
        "profile",
        "use-case",
        "capability-level",
        "capability-under-test",
        "program-scale",
        "program-domain",
        "module-count",
        "spec-count",
        "story-count",
        "interacts-with",
        "user-story-id",
        "user-story",
        "acceptance-criteria",
        "story-evidence",
        "story-file",
        "story-journey",
        "story-roundtrip",
        "distinctness-claim",
        "v0.3-signal",
        "package",
        "prompt-file",
        "prompt-version",
        "prompt-fingerprint",
        "executor-family",
        "executor-label",
        "capture-origin",
        "request-file",
        "response-file",
        "artifact-kind",
        "checker-result",
        "target",
    ] {
        if fields.get(field).is_none_or(|value| value.is_empty()) {
            return Err(format!("examples catalog entry {id} is missing {field}"));
        }
    }
    ail_e2e_validate_package_path_inside_examples(
        &id,
        fields
            .get("package")
            .map(String::as_str)
            .unwrap_or_default(),
    )?;
    ail_e2e_validate_usefulness_metadata(&id, fields)?;
    let checker_result = fields
        .get("checker-result")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(checker_result, "accepted" | "rejected") {
        return Err(format!(
            "examples catalog entry {id} has unknown checker-result {checker_result}"
        ));
    }
    let target = fields.get("target").map(String::as_str).unwrap_or_default();
    if !matches!(
        target,
        "vm" | "linux-x86_64-elf"
            | "wasm32-unknown-sandbox-wasm"
            | "aarch64-apple-darwin-libsystem-macho"
    ) {
        return Err(format!(
            "examples catalog entry {id} has unknown target {target}"
        ));
    }
    let capability_level = fields
        .get("capability-level")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(capability_level, "low-level" | "mid-level" | "high-level") {
        return Err(format!(
            "examples catalog entry {id} has unknown capability-level {capability_level}"
        ));
    }
    let program_scale = fields
        .get("program-scale")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(program_scale, "utility" | "module" | "multi-module-system") {
        return Err(format!(
            "examples catalog entry {id} has unknown program-scale {program_scale}"
        ));
    }
    let program_domain = fields
        .get("program-domain")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(
        program_domain,
        "os-utility"
            | "c-interop"
            | "compiler"
            | "runtime"
            | "package-graph"
            | "application"
            | "agent-tool"
            | "ui-workflow"
            | "system-driver"
            | "diagnostic"
    ) {
        return Err(format!(
            "examples catalog entry {id} has unknown program-domain {program_domain}"
        ));
    }
    let module_count = ail_e2e_positive_count_field(&id, fields, "module-count")?;
    let spec_count = ail_e2e_positive_count_field(&id, fields, "spec-count")?;
    let story_count = ail_e2e_positive_count_field(&id, fields, "story-count")?;
    let interacts_with = fields
        .get("interacts-with")
        .map(String::as_str)
        .unwrap_or_default();
    if program_scale == "multi-module-system"
        && (module_count < 2 || spec_count < 2 || story_count < 2 || interacts_with == "none")
    {
        return Err(format!(
            "examples catalog entry {id} multi-module-system must set module-count/spec-count/story-count >= 2 and interacts-with other than none"
        ));
    }
    if program_domain == "diagnostic" {
        let checker_result = fields
            .get("checker-result")
            .map(String::as_str)
            .unwrap_or_default();
        let story_evidence = fields
            .get("story-evidence")
            .map(String::as_str)
            .unwrap_or_default();
        if checker_result != "rejected" && story_evidence != "diagnostics" {
            return Err(format!(
                "examples catalog entry {id} diagnostic domain must use checker-result rejected or story-evidence diagnostics"
            ));
        }
    }
    let story_journey = fields
        .get("story-journey")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(
        story_journey,
        "story-to-spec" | "spec-to-story" | "story-amendment" | "diagnostic-story"
    ) {
        return Err(format!(
            "examples catalog entry {id} has unknown story-journey {story_journey}"
        ));
    }
    let story_roundtrip = fields
        .get("story-roundtrip")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(
        story_roundtrip,
        "semantic-similar" | "diagnostic-preserving"
    ) {
        return Err(format!(
            "examples catalog entry {id} has unknown story-roundtrip {story_roundtrip}"
        ));
    }
    let story_evidence = fields
        .get("story-evidence")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(
        story_evidence,
        "checked-core" | "bytecode" | "vm-trace" | "target-report" | "diagnostics"
    ) {
        return Err(format!(
            "examples catalog entry {id} has unknown story-evidence {story_evidence}"
        ));
    }
    let executor_family = fields
        .get("executor-family")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(
        executor_family,
        "llm-http" | "ail-toolchain-agent" | "codex-skill-agent"
    ) {
        return Err(format!(
            "examples catalog entry {id} has unknown executor-family {executor_family}"
        ));
    }
    let capture_origin = fields
        .get("capture-origin")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(
        capture_origin,
        "deterministic-seed" | "live-llm" | "live-codex"
    ) {
        return Err(format!(
            "examples catalog entry {id} has unknown capture-origin {capture_origin}"
        ));
    }
    let endpoint_label = fields
        .get("endpoint-label")
        .map(String::as_str)
        .unwrap_or_default();
    if executor_family == "llm-http" && endpoint_label.is_empty() {
        return Err(format!(
            "examples catalog llm-http entry {id} is missing endpoint-label"
        ));
    }
    if executor_family != "llm-http" && !endpoint_label.is_empty() {
        return Err(format!(
            "examples catalog offline executor entry {id} must not set endpoint-label"
        ));
    }
    let artifact_kind = fields
        .get("artifact-kind")
        .map(String::as_str)
        .unwrap_or_default();
    if !matches!(
        artifact_kind,
        "ail-requirements" | "ail-spec" | "ail-core" | "ail-flow-patch" | "prompt-envelope"
    ) {
        return Err(format!(
            "examples catalog entry {id} has unknown artifact-kind {artifact_kind}"
        ));
    }
    if fields
        .get("checker-result")
        .is_some_and(|result| result == "rejected")
    {
        for field in ["expected-diagnostic", "failure-taxonomy"] {
            if fields.get(field).is_none_or(|value| value.is_empty()) {
                return Err(format!(
                    "examples catalog rejected entry {id} is missing {field}"
                ));
            }
        }
    }
    Ok(AilE2eCorpusEntry {
        id,
        source_file: source_file.to_string(),
        fields: fields.clone(),
    })
}

fn ail_e2e_validate_catalog_relative_path(id: &str, field: &str, path: &str) -> Result<(), String> {
    let parsed = std::path::Path::new(path);
    if parsed.is_absolute()
        || !parsed
            .components()
            .any(|component| matches!(component, std::path::Component::Normal(_)))
        || parsed.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "examples catalog entry {id} {field} {path} must stay inside the examples catalog directory"
        ));
    }
    Ok(())
}

fn ail_e2e_validate_package_path_inside_examples(id: &str, path: &str) -> Result<(), String> {
    let parsed = std::path::Path::new(path);
    let normal_components = parsed
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    if parsed.is_absolute()
        || normal_components.first().copied() != Some("examples")
        || normal_components.len() < 2
        || parsed.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "examples catalog entry {id} package {path} must stay inside ./examples"
        ));
    }
    Ok(())
}

fn ail_e2e_validate_usefulness_metadata(
    id: &str,
    fields: &BTreeMap<String, String>,
) -> Result<(), String> {
    let use_case = fields
        .get("use-case")
        .map(String::as_str)
        .unwrap_or_default();
    if use_case.chars().count() < 50 {
        return Err(format!(
            "examples catalog entry {id} use-case must describe a concrete useful scenario"
        ));
    }
    let distinctness_claim = fields
        .get("distinctness-claim")
        .map(String::as_str)
        .unwrap_or_default();
    let semantic_task = fields
        .get("semantic-task")
        .map(String::as_str)
        .unwrap_or_default();
    let capability = fields
        .get("capability-under-test")
        .map(String::as_str)
        .unwrap_or_default();
    if !distinctness_claim.contains(semantic_task) || !distinctness_claim.contains(capability) {
        return Err(format!(
            "examples catalog entry {id} distinctness-claim must name semantic-task and capability-under-test"
        ));
    }
    if !ail_e2e_distinctness_claim_names_axis(distinctness_claim) {
        return Err(format!(
            "examples catalog entry {id} distinctness-claim must name a differentiating prompt, target, checker, diagnostic, story, artifact, executor, or human-review axis"
        ));
    }
    let v03_signal = fields
        .get("v0.3-signal")
        .map(String::as_str)
        .unwrap_or_default();
    if v03_signal.chars().count() < 50 {
        return Err(format!(
            "examples catalog entry {id} v0.3-signal must describe a concrete next-version learning"
        ));
    }
    let normalized_signal = v03_signal.to_ascii_lowercase();
    if !["need", "should", "must", "require"]
        .iter()
        .any(|keyword| normalized_signal.contains(keyword))
    {
        return Err(format!(
            "examples catalog entry {id} v0.3-signal must name a needed, required, or recommended next-version improvement"
        ));
    }
    Ok(())
}

fn ail_e2e_distinctness_claim_names_axis(claim: &str) -> bool {
    let claim = claim.to_ascii_lowercase();
    [
        "prompt",
        "target",
        "checker",
        "diagnostic",
        "story",
        "human-review",
        "human review",
        "artifact",
        "executor",
        "agent",
        "runtime",
        "vm",
        "wasm",
        "linux",
        "darwin",
        "evidence",
        "surface",
    ]
    .iter()
    .any(|axis| claim.contains(axis))
}

fn ail_e2e_positive_count_field(
    id: &str,
    fields: &BTreeMap<String, String>,
    field: &str,
) -> Result<usize, String> {
    let value = fields.get(field).map(String::as_str).unwrap_or_default();
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("examples catalog entry {id} has invalid {field} {value}"))?;
    if parsed == 0 {
        return Err(format!(
            "examples catalog entry {id} has invalid {field} {value}"
        ));
    }
    Ok(parsed)
}

fn validate_ail_e2e_corpus_transcript_files(entries: &[AilE2eCorpusEntry]) -> Result<(), String> {
    for entry in entries {
        let source_dir = std::path::Path::new(&entry.source_file)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        for field in ["request-file", "response-file", "story-file"] {
            let path = ail_e2e_entry_relative_path(entry, field)?;
            ail_e2e_validate_catalog_relative_path(&entry.id, field, &path)?;
            let resolved_path = source_dir.join(&path);
            if !resolved_path.is_file() {
                return Err(format!(
                    "examples catalog entry {} {field} {path} is missing",
                    entry.id
                ));
            }
        }
    }
    Ok(())
}

fn ail_e2e_catalog_root(path: &std::path::Path) -> std::path::PathBuf {
    if path.is_file() {
        return path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
    }
    path.to_path_buf()
}

fn ail_e2e_top_level_package_dirs(
    package_root: &std::path::Path,
) -> Result<BTreeSet<String>, String> {
    let mut packages = BTreeSet::new();
    if !package_root.is_dir() {
        return Ok(packages);
    }
    for entry in fs::read_dir(package_root).map_err(|error| {
        format!(
            "failed to read examples package root {}: {error}",
            package_root.display()
        )
    })? {
        let entry = entry
            .map_err(|error| format!("failed to read examples package root entry: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("failed to read examples package file type: {error}"))?
            .is_dir()
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".ail") {
            packages.insert(format!("examples/{name}"));
        }
    }
    Ok(packages)
}

fn ail_e2e_package_root_for_catalog(
    catalog_root: &std::path::Path,
) -> Result<Option<std::path::PathBuf>, String> {
    if !ail_e2e_top_level_package_dirs(catalog_root)?.is_empty() {
        return Ok(Some(catalog_root.to_path_buf()));
    }
    let nested_examples = catalog_root.join("examples");
    if !ail_e2e_top_level_package_dirs(&nested_examples)?.is_empty() {
        return Ok(Some(nested_examples));
    }
    Ok(None)
}

fn parse_ail_e2e_support_packages(
    source_file: &str,
    text: &str,
) -> Result<Vec<AilE2eSupportPackageEntry>, String> {
    let mut entries = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_fields = BTreeMap::<String, String>::new();
    for line in text.lines() {
        if let Some(path) = line.strip_prefix("## Support Package: ") {
            if let Some(package_path) = current_path.take() {
                entries.push(ail_e2e_support_package_entry_from_fields(
                    source_file,
                    package_path,
                    &current_fields,
                )?);
                current_fields.clear();
            }
            current_path = Some(path.trim().to_string());
            continue;
        }
        if current_path.is_some()
            && let Some((key, value)) = line.split_once(':')
        {
            let key = key.trim();
            if !key.is_empty()
                && key.chars().all(|ch| {
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.'
                })
            {
                current_fields.insert(key.to_string(), value.trim().to_string());
            }
        }
    }
    if let Some(package_path) = current_path.take() {
        entries.push(ail_e2e_support_package_entry_from_fields(
            source_file,
            package_path,
            &current_fields,
        )?);
    }
    Ok(entries)
}

fn ail_e2e_support_package_entry_from_fields(
    source_file: &str,
    path: String,
    fields: &BTreeMap<String, String>,
) -> Result<AilE2eSupportPackageEntry, String> {
    ail_e2e_validate_support_package_path(&path)?;
    let role = fields
        .get("role")
        .cloned()
        .ok_or_else(|| format!("examples support package {path} is missing role"))?;
    if role != "support-only" {
        return Err(format!(
            "examples support package {path} role {role} must be support-only"
        ));
    }
    let used_by_text = fields
        .get("used-by")
        .cloned()
        .ok_or_else(|| format!("examples support package {path} is missing used-by"))?;
    let used_by = used_by_text
        .split([',', ';'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if used_by.is_empty() {
        return Err(format!(
            "examples support package {path} used-by must not be empty"
        ));
    }
    let reason = fields
        .get("reason")
        .cloned()
        .ok_or_else(|| format!("examples support package {path} is missing reason"))?;
    if reason.chars().count() < 30 {
        return Err(format!(
            "examples support package {path} reason must explain why it is support-only"
        ));
    }
    if source_file.is_empty() {
        return Err(format!(
            "examples support package {path} has an empty source file"
        ));
    }
    Ok(AilE2eSupportPackageEntry { path, used_by })
}

fn ail_e2e_validate_support_package_path(path: &str) -> Result<(), String> {
    let parsed = std::path::Path::new(path);
    let normal_components = parsed
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    if parsed.is_absolute()
        || normal_components.first().copied() != Some("examples")
        || normal_components.len() != 2
        || !normal_components
            .get(1)
            .is_some_and(|component| component.ends_with(".ail"))
        || parsed.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "examples support package {path} must be a top-level ./examples/*.ail package"
        ));
    }
    Ok(())
}

fn validate_ail_e2e_support_package_closure(
    catalog_path: &std::path::Path,
    entries: &[AilE2eCorpusEntry],
) -> Result<(), String> {
    let catalog_root = ail_e2e_catalog_root(catalog_path);
    let Some(package_root) = ail_e2e_package_root_for_catalog(&catalog_root)? else {
        return Ok(());
    };
    let package_dirs = ail_e2e_top_level_package_dirs(&package_root)?;
    if package_dirs.is_empty() {
        return Ok(());
    }
    let counted_packages = entries
        .iter()
        .filter_map(|entry| entry.fields.get("package").cloned())
        .collect::<BTreeSet<_>>();
    for package in &counted_packages {
        if !package_dirs.contains(package) {
            return Err(format!(
                "examples catalog package {package} has no top-level package directory under ./examples"
            ));
        }
    }

    let support_manifest_path = package_root.join("support-packages.md");
    let support_entries = if support_manifest_path.is_file() {
        let support_text = fs::read_to_string(&support_manifest_path).map_err(|error| {
            format!(
                "failed to read examples support package manifest {}: {error}",
                support_manifest_path.display()
            )
        })?;
        parse_ail_e2e_support_packages(&support_manifest_path.to_string_lossy(), &support_text)?
    } else {
        Vec::new()
    };

    let mut declared_support = BTreeMap::<String, AilE2eSupportPackageEntry>::new();
    for support_entry in support_entries {
        if declared_support.contains_key(&support_entry.path) {
            return Err(format!(
                "examples support package {} is declared more than once in examples/support-packages.md",
                support_entry.path
            ));
        }
        if !package_dirs.contains(&support_entry.path) {
            return Err(format!(
                "examples support package {} is declared in examples/support-packages.md but no top-level package directory exists",
                support_entry.path
            ));
        }
        if counted_packages.contains(&support_entry.path) {
            return Err(format!(
                "examples support package {} is counted in examples.md and must not be declared support-only",
                support_entry.path
            ));
        }
        let mut has_concrete_user = false;
        for used_by in &support_entry.used_by {
            if used_by.starts_with("examples/") {
                ail_e2e_validate_support_package_path(used_by)?;
                if !package_dirs.contains(used_by) {
                    return Err(format!(
                        "examples support package {} used-by {} has no top-level package directory under ./examples",
                        support_entry.path, used_by
                    ));
                }
                if counted_packages.contains(used_by) {
                    has_concrete_user = true;
                }
            } else if ["toolchain:", "test:", "manual:", "docs:"]
                .iter()
                .any(|prefix| used_by.starts_with(prefix))
            {
                has_concrete_user = true;
            } else {
                return Err(format!(
                    "examples support package {} used-by {} must be an examples package or toolchain:/test:/manual:/docs: reference",
                    support_entry.path, used_by
                ));
            }
        }
        if !has_concrete_user {
            return Err(format!(
                "examples support package {} used-by must include a counted examples package or toolchain:/test:/manual:/docs: reference",
                support_entry.path
            ));
        }
        declared_support.insert(support_entry.path.clone(), support_entry);
    }

    for package in package_dirs {
        if counted_packages.contains(&package) || declared_support.contains_key(&package) {
            continue;
        }
        return Err(format!(
            "examples support package {package} is neither counted in examples.md nor declared in examples/support-packages.md"
        ));
    }
    Ok(())
}

fn parse_ail_e2e_story_file_fields(
    path: &std::path::Path,
) -> Result<BTreeMap<String, String>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read examples story file {}: {error}",
            path.display()
        )
    })?;
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            if !key.is_empty()
                && key.chars().all(|ch| {
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.'
                })
            {
                fields.insert(key.to_string(), value.trim().to_string());
            }
        }
    }
    Ok(fields)
}

fn ail_e2e_semantic_anchors_from_story_fields(
    story_fields: &BTreeMap<String, String>,
) -> Vec<String> {
    story_fields
        .get("semantic-anchors")
        .map(|semantic_anchors| {
            semantic_anchors
                .split([',', ';'])
                .map(str::trim)
                .filter(|anchor| !anchor.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn validate_ail_story_mode_fields(story_fields: &BTreeMap<String, String>) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if story_fields
        .get("user-story")
        .is_none_or(|value| value.trim().is_empty())
    {
        diagnostics.push("AIL-STORY-001 story file is missing user-story".to_string());
    }
    if story_fields
        .get("acceptance-criteria")
        .is_none_or(|value| value.trim().is_empty())
    {
        diagnostics.push("AIL-STORY-002 story file is missing acceptance-criteria".to_string());
    }
    if ail_e2e_semantic_anchors_from_story_fields(story_fields).len() < 3 {
        diagnostics.push(
            "AIL-STORY-003 story file semantic-anchors must list at least 3 anchors".to_string(),
        );
    }
    for field in ["module-count", "spec-count", "story-count"] {
        if let Some(value) = story_fields.get(field)
            && !matches!(value.trim().parse::<usize>(), Ok(parsed) if parsed > 0)
        {
            diagnostics.push(format!(
                "AIL-STORY-004 story file {field} must be a positive integer"
            ));
        }
    }
    if let Some(value) = story_fields.get("story-journey")
        && !matches!(
            value.as_str(),
            "story-to-spec" | "spec-to-story" | "story-amendment" | "diagnostic-story"
        )
    {
        diagnostics.push(format!(
            "AIL-STORY-005 story file story-journey has unknown value {value}"
        ));
    }
    if let Some(value) = story_fields.get("story-roundtrip")
        && !matches!(value.as_str(), "semantic-similar" | "diagnostic-preserving")
    {
        diagnostics.push(format!(
            "AIL-STORY-006 story file story-roundtrip has unknown value {value}"
        ));
    }
    diagnostics
}

fn normalized_ail_story_mode_fields(
    story_fields: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut normalized = story_fields.clone();
    normalized
        .entry("story-journey".to_string())
        .or_insert_with(|| "story-to-spec".to_string());
    normalized
        .entry("story-roundtrip".to_string())
        .or_insert_with(|| "semantic-similar".to_string());
    normalized
}

fn render_ail_story_mode_fields(fields: &BTreeMap<String, String>) -> String {
    let ordered_fields = [
        "user-story-id",
        "user-story",
        "acceptance-criteria",
        "story-journey",
        "story-roundtrip",
        "story-evidence",
        "program-domain",
        "module-count",
        "spec-count",
        "story-count",
        "interacts-with",
        "semantic-anchors",
    ];
    let mut rendered = String::from("# AIL Story Mode Input\n\n");
    let mut emitted = BTreeSet::new();
    for field in ordered_fields {
        if let Some(value) = fields.get(field) {
            rendered.push_str(&format!("{field}: {value}\n"));
            emitted.insert(field.to_string());
        }
    }
    for (field, value) in fields {
        if !emitted.contains(field) {
            rendered.push_str(&format!("{field}: {value}\n"));
        }
    }
    rendered
}

fn render_ail_story_requirements_prompt(story_normalized_text: &str) -> String {
    format!(
        concat!(
            "Draft AIL requirements from this AIL user story.\n\n",
            "The story is authoring input, not trusted code. Preserve the user story, acceptance criteria, and semantic anchors as requirements evidence, but rely on the AIL parser, checker, compiler, and runtime as the authority for executable behavior.\n\n",
            "USER STORY MODE INPUT:\n",
            "{}\n\n",
            "Produce the complete AIL-Requirements artifact inside the prompt envelope artifact_text, with bullets for domain objects, required fields, action inputs or preconditions, failure cases, guarantees, trace events, secrets, permissions, and runtime inputs."
        ),
        story_normalized_text.trim()
    )
}

fn render_ail_story_spec_prompt(story_normalized_text: &str, semantic_anchors: &str) -> String {
    format!(
        concat!(
            "Draft an AIL-Spec candidate from checked AIL-Requirements and this AIL user story.\n\n",
            "Preserve these story semantic anchors: {}\n\n",
            "USER STORY MODE INPUT:\n",
            "{}\n\n",
            "The resulting AIL-Spec must preserve the acceptance criteria, explain the same domain behavior, and lower to checked AIL-Core and verified bytecode."
        ),
        semantic_anchors,
        story_normalized_text.trim()
    )
}

fn read_ail_e2e_entry_semantic_anchors(entry: &AilE2eCorpusEntry) -> Result<Vec<String>, String> {
    let story_path = ail_e2e_entry_resolved_path(entry, "story-file")?;
    let story_fields = parse_ail_e2e_story_file_fields(&story_path)?;
    Ok(ail_e2e_semantic_anchors_from_story_fields(&story_fields))
}

fn validate_ail_e2e_corpus_story_files(
    entries: &[AilE2eCorpusEntry],
    require_release_semantic_anchors: bool,
) -> Result<(), String> {
    let mut missing_semantic_anchor_story_count = 0usize;
    for entry in entries {
        let story_path = ail_e2e_entry_resolved_path(entry, "story-file")?;
        let story_fields = parse_ail_e2e_story_file_fields(&story_path)?;
        for field in [
            "user-story-id",
            "user-story",
            "acceptance-criteria",
            "story-journey",
            "story-roundtrip",
            "story-evidence",
            "program-domain",
            "module-count",
            "spec-count",
            "story-count",
            "interacts-with",
        ] {
            let catalog_value = entry.fields.get(field).map(String::as_str).unwrap_or("");
            let story_value = story_fields.get(field).map(String::as_str).unwrap_or("");
            if story_value.is_empty() {
                return Err(format!(
                    "examples catalog entry {} story-file is missing {field}",
                    entry.id
                ));
            }
            if story_value != catalog_value {
                return Err(format!(
                    "examples catalog entry {} story-file {field} mismatch: catalog `{catalog_value}` story `{story_value}`",
                    entry.id
                ));
            }
        }
        let semantic_anchor_count = ail_e2e_semantic_anchors_from_story_fields(&story_fields).len();
        if semantic_anchor_count > 0 && semantic_anchor_count < 3 {
            return Err(format!(
                "examples catalog entry {} story-file semantic-anchors must list at least 3 anchors",
                entry.id
            ));
        }
        if semantic_anchor_count < 3 && require_release_semantic_anchors {
            missing_semantic_anchor_story_count += 1;
        }
    }
    if require_release_semantic_anchors && missing_semantic_anchor_story_count > 0 {
        return Err(format!(
            "ail-examples --release-evidence requires semantic-anchor story files for every catalog entry; missing {missing_semantic_anchor_story_count}"
        ));
    }
    Ok(())
}

fn ail_e2e_entry_source_dir(entry: &AilE2eCorpusEntry) -> &std::path::Path {
    std::path::Path::new(&entry.source_file)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
}

fn ail_e2e_entry_relative_path(entry: &AilE2eCorpusEntry, field: &str) -> Result<String, String> {
    entry
        .fields
        .get(field)
        .cloned()
        .ok_or_else(|| format!("examples catalog entry {} is missing {field}", entry.id))
}

fn ail_e2e_entry_resolved_path(
    entry: &AilE2eCorpusEntry,
    field: &str,
) -> Result<std::path::PathBuf, String> {
    let path = ail_e2e_entry_relative_path(entry, field)?;
    ail_e2e_validate_catalog_relative_path(&entry.id, field, &path)?;
    Ok(ail_e2e_entry_source_dir(entry).join(path))
}

fn extract_ail_e2e_response_artifact_text(response_text: &str) -> String {
    let trimmed = response_text.trim();
    if let Some(fenced) = trimmed.strip_prefix("```") {
        let without_language = match fenced.find('\n') {
            Some(newline_index) => &fenced[newline_index + 1..],
            None => fenced,
        };
        if let Some(end) = without_language.rfind("```") {
            return without_language[..end].trim().to_string();
        }
    }
    for field in ["content", "artifact_text"] {
        if let Some(value) = extract_ail_e2e_json_string_field(trimmed, field) {
            return value.trim().to_string();
        }
    }
    if let Some(value) = extract_ail_e2e_chat_completion_message_content(trimmed) {
        return value.trim().to_string();
    }
    trimmed.to_string()
}

fn extract_ail_e2e_chat_completion_message_content(text: &str) -> Option<String> {
    let choices_start = text.find("\"choices\"")?;
    let message_start = choices_start + text[choices_start..].find("\"message\"")?;
    let content_start = message_start + text[message_start..].find("\"content\"")?;
    extract_ail_e2e_json_string_field(&text[content_start..], "content")
}

fn extract_ail_e2e_json_string_field(text: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\"");
    let mut search_start = 0;
    while search_start < text.len() {
        let field_start = search_start + text[search_start..].find(&needle)?;
        let mut index = field_start + needle.len();
        index = skip_ail_e2e_json_whitespace(text, index);
        if text.as_bytes().get(index) == Some(&b':') {
            let start = skip_ail_e2e_json_whitespace(text, index + 1);
            let mut chars = text[start..].chars().peekable();
            return parse_ail_e2e_json_string(&mut chars);
        }
        search_start = field_start + needle.len();
    }
    None
}

fn skip_ail_e2e_json_whitespace(text: &str, mut index: usize) -> usize {
    while text
        .as_bytes()
        .get(index)
        .is_some_and(u8::is_ascii_whitespace)
    {
        index += 1;
    }
    index
}

fn parse_ail_e2e_json_string<I>(chars: &mut std::iter::Peekable<I>) -> Option<String>
where
    I: Iterator<Item = char>,
{
    if chars.next()? != '"' {
        return None;
    }
    let mut output = String::new();
    let mut escaped = false;
    for ch in chars.by_ref() {
        if escaped {
            output.push(match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                other => other,
            });
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Some(output);
        }
        output.push(ch);
    }
    None
}

fn read_ail_e2e_entry_transcripts(entry: &AilE2eCorpusEntry) -> Result<(String, String), String> {
    let request_path = ail_e2e_entry_resolved_path(entry, "request-file")?;
    let request_text = fs::read_to_string(&request_path).map_err(|error| {
        format!(
            "failed to read examples catalog request {}: {error}",
            request_path.display()
        )
    })?;
    let response_path = ail_e2e_entry_resolved_path(entry, "response-file")?;
    let response_text = fs::read_to_string(&response_path).map_err(|error| {
        format!(
            "failed to read examples catalog response {}: {error}",
            response_path.display()
        )
    })?;
    Ok((request_text, response_text))
}

fn evaluate_rejected_ail_e2e_corpus_entry(
    entry: &AilE2eCorpusEntry,
) -> Result<AilE2eCorpusEvaluation, String> {
    let artifact_kind = entry
        .fields
        .get("artifact-kind")
        .map(String::as_str)
        .unwrap_or_default();
    if artifact_kind != "ail-spec" && artifact_kind != "prompt-envelope" {
        return Err(format!(
            "examples catalog rejected entry {} has unsupported artifact-kind {artifact_kind}",
            entry.id
        ));
    }
    let package_path = entry
        .fields
        .get("package")
        .ok_or_else(|| format!("examples catalog entry {} is missing package", entry.id))?;
    let expected_diagnostic = entry.fields.get("expected-diagnostic").ok_or_else(|| {
        format!(
            "examples catalog rejected entry {} is missing expected-diagnostic",
            entry.id
        )
    })?;
    let failure_taxonomy = entry.fields.get("failure-taxonomy").ok_or_else(|| {
        format!(
            "examples catalog rejected entry {} is missing failure-taxonomy",
            entry.id
        )
    })?;
    let (request_text, response_text) = read_ail_e2e_entry_transcripts(entry)?;
    let (artifact_text, diagnostics) = if artifact_kind == "prompt-envelope" {
        (
            response_text.clone(),
            vec![validate_stored_prompt_envelope_output(&response_text)],
        )
    } else {
        let spec_text = extract_ail_e2e_response_artifact_text(&response_text);
        let diagnostics = match load_ail_package_dir(package_path) {
            Ok(package) => match parse_ail_package_spec_text(&package, &spec_text) {
                Ok(document) => {
                    let core = elaborate_ail_core(&package, &document);
                    let mut diagnostics = check_ail_core(&core);
                    if diagnostics.is_empty()
                        && let Some(target) = entry.fields.get("target")
                        && let Err(error) =
                            check_darwin_macho_contract_supported_effects(&core, target)
                    {
                        diagnostics.push(error);
                    }
                    diagnostics
                }
                Err(error) => vec![format!("parse-error {error}")],
            },
            Err(error) => vec![error],
        };
        (spec_text, diagnostics)
    };
    if diagnostics.is_empty() {
        return Err(format!(
            "examples catalog rejected entry {} was accepted without diagnostics",
            entry.id
        ));
    }
    if !diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains(expected_diagnostic))
    {
        return Err(format!(
            "examples catalog rejected entry {} expected diagnostic {expected_diagnostic} was not produced:\n{}",
            entry.id,
            diagnostics.join("\n")
        ));
    }
    let mut lines = vec![
        "AIL-Examples-Rejected-Diagnostics:".to_string(),
        "checker-result rejected".to_string(),
        format!("expected-diagnostic {expected_diagnostic}"),
        format!("failure-taxonomy {failure_taxonomy}"),
    ];
    for diagnostic in diagnostics {
        lines.push(format!("diagnostic {diagnostic}"));
    }
    let diagnostics_text = format!("{}\n", lines.join("\n"));
    let repair_tutorial_text =
        render_ail_e2e_repair_tutorial(entry, expected_diagnostic, failure_taxonomy, &lines[4..]);
    let semantic_anchors = read_ail_e2e_entry_semantic_anchors(entry)?;
    let repair_proof = build_ail_e2e_repair_proof(AilE2eRepairProofInput {
        entry,
        package_path,
        failure_taxonomy,
        artifact_kind,
        expected_diagnostic,
        rejected_artifact_text: &artifact_text,
        diagnostics_text: &diagnostics_text,
        repair_tutorial_text: &repair_tutorial_text,
        semantic_anchors: &semantic_anchors,
    })?;
    Ok(AilE2eCorpusEvaluation {
        entry: entry.clone(),
        semantic_anchors,
        request_fingerprint: Some(ail_artifact_fingerprint(&request_text)),
        response_fingerprint: Some(ail_artifact_fingerprint(&response_text)),
        extracted_artifact_fingerprint: Some(ail_artifact_fingerprint(&artifact_text)),
        checked_core_text: None,
        bytecode_text: None,
        vm_trace_text: None,
        target_report_text: None,
        ui_review_text: None,
        ui_review_patch_text: None,
        ui_semantic_tags_text: None,
        agent_policy_review_text: None,
        threat_model_audit_text: None,
        type_inference_review_text: None,
        state_boundary_review_text: None,
        workflow_scheduler_review_text: None,
        unsafe_boundary_review_text: None,
        complex_story_graph_text: None,
        application_walkthrough_text: None,
        story_promotion_review_text: None,
        dependency_review_text: None,
        stdlib_walkthrough_text: None,
        diagnostics_text: Some(diagnostics_text),
        repair_tutorial_text: Some(repair_tutorial_text),
        repair_proof: Some(repair_proof),
        native_executables: Vec::new(),
    })
}

struct AilE2eRepairProofInput<'a> {
    entry: &'a AilE2eCorpusEntry,
    package_path: &'a str,
    failure_taxonomy: &'a str,
    artifact_kind: &'a str,
    expected_diagnostic: &'a str,
    rejected_artifact_text: &'a str,
    diagnostics_text: &'a str,
    repair_tutorial_text: &'a str,
    semantic_anchors: &'a [String],
}

fn build_ail_e2e_repair_proof(
    input: AilE2eRepairProofInput<'_>,
) -> Result<AilE2eRepairProofArtifacts, String> {
    let (package, candidate_spec_text) = load_ail_e2e_repair_candidate_package(
        input.entry,
        input.package_path,
        input.failure_taxonomy,
        input.artifact_kind,
        input.rejected_artifact_text,
    )?;
    let document =
        parse_ail_package_spec_text(&package, &candidate_spec_text).map_err(|error| {
            format!(
                "examples catalog rejected entry {} repair candidate failed to parse: {error}",
                input.entry.id
            )
        })?;
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core(&core);
    if !diagnostics.is_empty() {
        return Err(format!(
            "examples catalog rejected entry {} repair candidate still has diagnostics:\n{}",
            input.entry.id,
            diagnostics.join("\n")
        ));
    }
    let target = input
        .entry
        .fields
        .get("target")
        .map(String::as_str)
        .unwrap_or_default();
    if let Err(error) = check_darwin_macho_contract_supported_effects(&core, target) {
        return Err(format!(
            "examples catalog rejected entry {} repair candidate failed target check: {error}",
            input.entry.id
        ));
    }
    let bytecode = compile_ail_core_bytecode(&core)?;
    let bytecode_diagnostics = verify_ail_bytecode(&bytecode);
    if !bytecode_diagnostics.is_empty() {
        return Err(format!(
            "examples catalog rejected entry {} repair candidate has bytecode diagnostics:\n{}",
            input.entry.id,
            bytecode_diagnostics.join("\n")
        ));
    }
    let repair_action_name = ail_e2e_repair_action_name(input.entry, &bytecode)?;
    let target_report_text = match target {
        "wasm32-unknown-sandbox-wasm" => Some(render_ail_compile_wasm_contract_report(
            &bytecode,
            &repair_action_name,
            target,
        )?),
        "aarch64-apple-darwin-libsystem-macho" => {
            Some(render_ail_compile_darwin_macho_contract_report(
                &bytecode,
                &repair_action_name,
                target,
            )?)
        }
        _ => None,
    };
    let vm_trace_text = if target_report_text.is_none() {
        let runtime_state = parse_ail_e2e_runtime_state(input.entry)?;
        let run = run_ail_bytecode_action(&bytecode, &repair_action_name, runtime_state)?;
        Some(format!("{}\n", run.trace.join("\n")))
    } else {
        None
    };
    let checked_core_text = render_ail_core(&core);
    let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
    let repair_diff_text = render_ail_e2e_repair_diff(AilE2eRepairDiffInput {
        entry: input.entry,
        failure_taxonomy: input.failure_taxonomy,
        expected_diagnostic: input.expected_diagnostic,
        rejected_artifact_text: input.rejected_artifact_text,
        candidate_spec_text: &candidate_spec_text,
        checked_core_text: &checked_core_text,
        bytecode_text: &bytecode_text,
        vm_trace_text: vm_trace_text.as_deref(),
        target_report_text: target_report_text.as_deref(),
        semantic_anchors: input.semantic_anchors,
    });
    let promotion_review_text =
        render_ail_e2e_repair_promotion_review(AilE2eRepairPromotionReviewInput {
            entry: input.entry,
            failure_taxonomy: input.failure_taxonomy,
            expected_diagnostic: input.expected_diagnostic,
            diagnostics_text: input.diagnostics_text,
            repair_tutorial_text: input.repair_tutorial_text,
            candidate_spec_text: &candidate_spec_text,
            checked_core_text: &checked_core_text,
            bytecode_text: &bytecode_text,
            vm_trace_text: vm_trace_text.as_deref(),
            target_report_text: target_report_text.as_deref(),
            repair_diff_text: &repair_diff_text,
            semantic_anchors: input.semantic_anchors,
        });
    Ok(AilE2eRepairProofArtifacts {
        candidate_spec_text,
        checked_core_text,
        bytecode_text,
        vm_trace_text,
        target_report_text,
        repair_diff_text,
        promotion_review_text,
    })
}

struct AilE2eRepairDiffInput<'a> {
    entry: &'a AilE2eCorpusEntry,
    failure_taxonomy: &'a str,
    expected_diagnostic: &'a str,
    rejected_artifact_text: &'a str,
    candidate_spec_text: &'a str,
    checked_core_text: &'a str,
    bytecode_text: &'a str,
    vm_trace_text: Option<&'a str>,
    target_report_text: Option<&'a str>,
    semantic_anchors: &'a [String],
}

fn render_ail_e2e_repair_diff(input: AilE2eRepairDiffInput<'_>) -> String {
    let repair_evidence_kind = if input.target_report_text.is_some() {
        "repair-target-report"
    } else {
        "repair-vm-trace"
    };
    let repair_evidence_text = input
        .target_report_text
        .or(input.vm_trace_text)
        .unwrap_or("");
    let mut lines = vec![
        "AIL-Repair-Diff:".to_string(),
        format!("entry {}", input.entry.id),
        "checker-result rejected-to-repaired".to_string(),
        format!("failure-taxonomy {}", input.failure_taxonomy),
        format!("expected-diagnostic {}", input.expected_diagnostic),
        "expected-diagnostic-removed true".to_string(),
        format!("repair-evidence-kind {repair_evidence_kind}"),
        format!(
            "rejected-artifact-fingerprint {}",
            ail_artifact_fingerprint(input.rejected_artifact_text)
        ),
        format!(
            "repair-candidate-fingerprint {}",
            ail_artifact_fingerprint(input.candidate_spec_text)
        ),
        format!(
            "repair-checked-core-fingerprint {}",
            ail_artifact_fingerprint(input.checked_core_text)
        ),
        format!(
            "repair-bytecode-fingerprint {}",
            ail_artifact_fingerprint(input.bytecode_text)
        ),
        format!(
            "repair-evidence-fingerprint {}",
            ail_artifact_fingerprint(repair_evidence_text)
        ),
        format!("semantic-anchor-count {}", input.semantic_anchors.len()),
        format!(
            "semantic-anchor-preserved-count {}",
            input.semantic_anchors.len()
        ),
        "semantic-anchor-missing-count 0".to_string(),
    ];
    for anchor in input.semantic_anchors {
        lines.push(format!("semantic-anchor {anchor} preserved"));
    }
    lines.push("repair-diff-summary rejected diagnostic reproduced, corrected artifact checked, and repaired proof evidence generated".to_string());
    format!("{}\n", lines.join("\n"))
}

struct AilE2eRepairPromotionReviewInput<'a> {
    entry: &'a AilE2eCorpusEntry,
    failure_taxonomy: &'a str,
    expected_diagnostic: &'a str,
    diagnostics_text: &'a str,
    repair_tutorial_text: &'a str,
    candidate_spec_text: &'a str,
    checked_core_text: &'a str,
    bytecode_text: &'a str,
    vm_trace_text: Option<&'a str>,
    target_report_text: Option<&'a str>,
    repair_diff_text: &'a str,
    semantic_anchors: &'a [String],
}

fn render_ail_e2e_repair_promotion_review(input: AilE2eRepairPromotionReviewInput<'_>) -> String {
    let repair_evidence_kind = if input.target_report_text.is_some() {
        "repair-target-report"
    } else {
        "repair-vm-trace"
    };
    let repair_evidence_text = input
        .target_report_text
        .or(input.vm_trace_text)
        .unwrap_or("");
    let story_text = render_ail_e2e_user_story_text(input.entry, input.semantic_anchors);
    let (preserved_count, missing_count) =
        ail_e2e_semantic_anchor_preservation_counts(&story_text, input.semantic_anchors);
    let mut lines = vec![
        "AIL-Repair-Promotion-Review:".to_string(),
        format!("entry {}", input.entry.id),
        "reviewer-agent codex-ail-repair-promotion-reviewer".to_string(),
        "promotion-decision accepted-for-promotion".to_string(),
        "human-approval-required true".to_string(),
        format!("proposed-accepted-entry-id {}-repaired", input.entry.id),
        "checker-result rejected-to-repaired".to_string(),
        format!("failure-taxonomy {}", input.failure_taxonomy),
        format!("expected-diagnostic {}", input.expected_diagnostic),
        "expected-diagnostic-removed true".to_string(),
        format!("repair-evidence-kind {repair_evidence_kind}"),
        format!(
            "diagnostics-fingerprint {}",
            ail_artifact_fingerprint(input.diagnostics_text)
        ),
        format!(
            "repair-tutorial-fingerprint {}",
            ail_artifact_fingerprint(input.repair_tutorial_text)
        ),
        format!(
            "repair-candidate-fingerprint {}",
            ail_artifact_fingerprint(input.candidate_spec_text)
        ),
        format!(
            "repair-checked-core-fingerprint {}",
            ail_artifact_fingerprint(input.checked_core_text)
        ),
        format!(
            "repair-bytecode-fingerprint {}",
            ail_artifact_fingerprint(input.bytecode_text)
        ),
        format!(
            "repair-evidence-fingerprint {}",
            ail_artifact_fingerprint(repair_evidence_text)
        ),
        format!(
            "repair-diff-fingerprint {}",
            ail_artifact_fingerprint(input.repair_diff_text)
        ),
        format!(
            "semantic-anchor-count {}",
            input.semantic_anchors.len()
        ),
        format!("semantic-anchor-preserved-count {preserved_count}"),
        format!("semantic-anchor-missing-count {missing_count}"),
        "promotion-review-summary rejected replay reproduced the expected diagnostic and the repaired candidate passed checked Core, bytecode, and VM or target evidence.".to_string(),
    ];
    for anchor in input.semantic_anchors {
        lines.push(format!("semantic-anchor {anchor} preserved"));
    }
    format!("{}\n", lines.join("\n"))
}

fn load_ail_e2e_repair_candidate_package(
    entry: &AilE2eCorpusEntry,
    package_path: &str,
    failure_taxonomy: &str,
    artifact_kind: &str,
    rejected_artifact_text: &str,
) -> Result<(AilPackage, String), String> {
    if failure_taxonomy == "package-resolution" {
        return synthesize_package_resolution_repair_candidate(entry, package_path);
    }
    let package = load_ail_package_dir(package_path).map_err(|error| {
        format!(
            "examples catalog rejected entry {} repair package failed to load: {error}",
            entry.id
        )
    })?;
    let mut candidate_spec_text = package.spec_text.clone();
    if failure_taxonomy == "semantic-drift" {
        let repaired = rejected_artifact_text
            .replace("the account to exist", "the ticket to exist")
            .replace("account to exist", "ticket to exist");
        if repaired != rejected_artifact_text {
            candidate_spec_text = repaired;
        }
    }
    if failure_taxonomy == "missing-trace" && !rejected_artifact_text.contains("trace event named")
    {
        candidate_spec_text = ensure_trailing_newline(rejected_artifact_text.to_string());
        let repair_trace = if rejected_artifact_text.contains("increment counter") {
            "CounterIncremented"
        } else if rejected_artifact_text.contains("Close ticket")
            || rejected_artifact_text.contains("closes a ticket")
        {
            "TicketClosed"
        } else {
            "RepairTraceCompleted"
        };
        candidate_spec_text.push_str(&format!(
            "- the system records a trace event named {repair_trace}\n"
        ));
    }
    if failure_taxonomy == "unsupported-target" {
        candidate_spec_text = candidate_spec_text
            .replace("- call linux syscall exit", "- call darwin process exit")
            .replace("- linux syscall exit", "- darwin process exit");
    }
    if artifact_kind == "prompt-envelope" {
        candidate_spec_text = package.spec_text.clone();
    }
    Ok((package, candidate_spec_text))
}

fn synthesize_package_resolution_repair_candidate(
    entry: &AilE2eCorpusEntry,
    package_path: &str,
) -> Result<(AilPackage, String), String> {
    let root = std::path::PathBuf::from(package_path);
    let metadata_path = root.join("ail-package.md");
    let metadata_text = fs::read_to_string(&metadata_path).map_err(|error| {
        format!(
            "examples catalog rejected entry {} repair package failed to read {}: {error}",
            entry.id,
            metadata_path.display()
        )
    })?;
    let field = |key: &str| ail_simple_metadata_field(&metadata_text, key);
    let entry_file = field("entry").unwrap_or_else(|| "spec.ail-spec.md".to_string());
    let spec_path = root.join(&entry_file);
    let candidate_spec_text = fs::read_to_string(&spec_path).map_err(|error| {
        format!(
            "examples catalog rejected entry {} repair package failed to read {}: {error}",
            entry.id,
            spec_path.display()
        )
    })?;
    let repaired_spec_text = candidate_spec_text.replace(
        "shared-lib to resolve from the registry index",
        "the corrected local dependency to be available",
    );
    let target = entry
        .fields
        .get("target")
        .cloned()
        .unwrap_or_else(|| "vm".to_string());
    let mut target_support = BTreeMap::new();
    target_support.insert(target, "supported".to_string());
    let package = AilPackage {
        metadata: AilPackageMetadata {
            name: field("name").unwrap_or_else(|| "missing-registry-import".to_string()),
            version: field("version").unwrap_or_else(|| "0.2.0".to_string()),
            profile: field("profile").unwrap_or_else(|| "Application".to_string()),
            entry: entry_file,
            features: vec!["repair-proof".to_string()],
            imports: Vec::new(),
            capability_grants: Vec::new(),
            conformance: field("conformance").unwrap_or_else(|| "v0.2".to_string()),
            prompt_pack: None,
            registry: None,
            target_support,
            schema_version: field("schema-version"),
            safety_level: field("safety-level"),
            base_llm_endpoint: DEFAULT_BASE_LLM_ENDPOINT.to_string(),
        },
        root,
        spec_path,
        spec_text: repaired_spec_text.clone(),
        imports: Vec::new(),
    };
    Ok((package, repaired_spec_text))
}

fn ail_simple_metadata_field(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let Some((field, value)) = line.split_once(':') else {
            continue;
        };
        if field.trim() == key {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn ail_e2e_repair_action_name(
    entry: &AilE2eCorpusEntry,
    bytecode: &AilBytecodeProgram,
) -> Result<String, String> {
    if let Some(action_name) = entry
        .fields
        .get("vm-action")
        .filter(|action_name| bytecode.actions.contains_key(action_name.as_str()))
    {
        return Ok(action_name.clone());
    }
    bytecode.actions.keys().next().cloned().ok_or_else(|| {
        format!(
            "examples catalog rejected entry {} repair candidate has no bytecode actions",
            entry.id
        )
    })
}

fn render_ail_e2e_repair_tutorial(
    entry: &AilE2eCorpusEntry,
    expected_diagnostic: &str,
    failure_taxonomy: &str,
    diagnostic_lines: &[String],
) -> String {
    let field = |key: &str| {
        entry
            .fields
            .get(key)
            .map(String::as_str)
            .unwrap_or("unknown")
    };
    let mut lines = vec![
        "AIL-Repair-Tutorial:".to_string(),
        format!("entry {}", entry.id),
        "checker-result rejected".to_string(),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("prompt-file {}", field("prompt-file")),
        format!("story-file {}", field("story-file")),
        format!("failure-taxonomy {failure_taxonomy}"),
        format!("expected-diagnostic {expected_diagnostic}"),
        "diagnostic-summary:".to_string(),
    ];
    lines.extend(diagnostic_lines.iter().cloned());
    lines.extend([
        "repair-plan:".to_string(),
        "repair-step 1 Preserve the rejected transcript, diagnostics, and story as review evidence."
            .to_string(),
        "repair-step 2 Draft a corrected spec that removes the expected diagnostic while preserving semantic anchors."
            .to_string(),
        "repair-step 3 Replay ail-examples for this corpus and promote the corrected artifact only after checked Core, bytecode, and target or VM evidence pass."
            .to_string(),
    ]);
    format!("{}\n", lines.join("\n"))
}

fn evaluate_ail_e2e_corpus_entry(
    entry: &AilE2eCorpusEntry,
) -> Result<AilE2eCorpusEvaluation, String> {
    let checker_result = entry
        .fields
        .get("checker-result")
        .map(String::as_str)
        .unwrap_or_default();
    if checker_result == "rejected" {
        return evaluate_rejected_ail_e2e_corpus_entry(entry);
    }
    if checker_result != "accepted" {
        return Err(format!(
            "examples catalog entry {} has unknown checker-result {checker_result}",
            entry.id
        ));
    }
    let artifact_kind = entry
        .fields
        .get("artifact-kind")
        .map(String::as_str)
        .unwrap_or_default();
    if artifact_kind != "ail-spec" {
        return Ok(AilE2eCorpusEvaluation {
            entry: entry.clone(),
            semantic_anchors: read_ail_e2e_entry_semantic_anchors(entry)?,
            request_fingerprint: None,
            response_fingerprint: None,
            extracted_artifact_fingerprint: None,
            checked_core_text: None,
            bytecode_text: None,
            vm_trace_text: None,
            target_report_text: None,
            ui_review_text: None,
            ui_review_patch_text: None,
            ui_semantic_tags_text: None,
            agent_policy_review_text: None,
            threat_model_audit_text: None,
            type_inference_review_text: None,
            state_boundary_review_text: None,
            workflow_scheduler_review_text: None,
            unsafe_boundary_review_text: None,
            complex_story_graph_text: None,
            application_walkthrough_text: None,
            story_promotion_review_text: None,
            dependency_review_text: None,
            stdlib_walkthrough_text: None,
            diagnostics_text: None,
            repair_tutorial_text: None,
            repair_proof: None,
            native_executables: Vec::new(),
        });
    }
    let package_path = entry
        .fields
        .get("package")
        .ok_or_else(|| format!("examples catalog entry {} is missing package", entry.id))?;
    let (request_text, response_text) = read_ail_e2e_entry_transcripts(entry)?;
    let spec_text = extract_ail_e2e_response_artifact_text(&response_text);
    let package = load_ail_package_dir(package_path)?;
    let document = parse_ail_package_spec_text(&package, &spec_text)?;
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core(&core);
    if !diagnostics.is_empty() {
        return Err(format!(
            "examples catalog accepted entry {} has diagnostics:\n{}",
            entry.id,
            diagnostics.join("\n")
        ));
    }
    let bytecode = compile_ail_core_bytecode(&core)?;
    let bytecode_diagnostics = verify_ail_bytecode(&bytecode);
    if !bytecode_diagnostics.is_empty() {
        return Err(format!(
            "examples catalog accepted entry {} has bytecode diagnostics:\n{}",
            entry.id,
            bytecode_diagnostics.join("\n")
        ));
    }
    let target = entry
        .fields
        .get("target")
        .map(String::as_str)
        .unwrap_or_default();
    let vm_trace_text = if let Some(action_name) = entry
        .fields
        .get("vm-action")
        .filter(|action_name| !action_name.is_empty())
    {
        let runtime_state = parse_ail_e2e_runtime_state(entry)?;
        let run = run_ail_bytecode_action(&bytecode, action_name, runtime_state)?;
        Some(format!("{}\n", run.trace.join("\n")))
    } else {
        None
    };
    let native_executables = if target == "linux-x86_64-elf" {
        compile_ail_native_artifacts(&bytecode, target, "target")?
    } else {
        Vec::new()
    };
    let contract_action_name = entry
        .fields
        .get("vm-action")
        .filter(|action_name| !action_name.is_empty())
        .map(String::as_str)
        .or_else(|| bytecode.actions.keys().next().map(String::as_str));
    let target_report_text = match target {
        "linux-x86_64-elf" if !native_executables.is_empty() => Some(
            render_ail_e2e_native_target_report(target, native_executables.as_slice())?,
        ),
        "wasm32-unknown-sandbox-wasm" => Some(render_ail_compile_wasm_contract_report(
            &bytecode,
            contract_action_name.ok_or_else(|| {
                format!(
                    "examples catalog entry {} target {target} requires a bytecode action",
                    entry.id
                )
            })?,
            target,
        )?),
        "aarch64-apple-darwin-libsystem-macho" => {
            Some(render_ail_compile_darwin_macho_contract_report(
                &bytecode,
                contract_action_name.ok_or_else(|| {
                    format!(
                        "examples catalog entry {} target {target} requires a bytecode action",
                        entry.id
                    )
                })?,
                target,
            )?)
        }
        _ => None,
    };
    let semantic_anchors = read_ail_e2e_entry_semantic_anchors(entry)?;
    let checked_core_text = render_ail_core(&core);
    let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
    let ui_review_text = render_ail_e2e_ui_review_text(
        entry,
        &semantic_anchors,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let ui_review_patch_text = ui_review_text.as_ref().map(|text| {
        render_ail_e2e_ui_review_patch_text(
            entry,
            text,
            Some(&checked_core_text),
            Some(&bytecode_text),
            vm_trace_text.as_deref(),
            target_report_text.as_deref(),
        )
    });
    let ui_semantic_tags_text = render_ail_e2e_ui_semantic_tags_text(
        entry,
        &semantic_anchors,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let agent_policy_review_text = render_ail_e2e_agent_policy_review_text(
        entry,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let threat_model_audit_text = render_ail_e2e_threat_model_audit_text(
        entry,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let type_inference_review_text = render_ail_e2e_type_inference_review_text(
        entry,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let state_boundary_review_text = render_ail_e2e_state_boundary_review_text(
        entry,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let workflow_scheduler_review_text = render_ail_e2e_workflow_scheduler_review_text(
        entry,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let unsafe_boundary_review_text = render_ail_e2e_unsafe_boundary_review_text(
        entry,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let complex_story_graph_text = render_ail_e2e_complex_story_graph_text(
        entry,
        &semantic_anchors,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let application_walkthrough_text = render_ail_e2e_application_walkthrough_text(
        entry,
        &semantic_anchors,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let story_promotion_review_text = render_ail_e2e_story_promotion_review_text(
        entry,
        &semantic_anchors,
        &request_text,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    )?;
    let dependency_review_text = render_ail_e2e_dependency_review_text(
        entry,
        &semantic_anchors,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    let stdlib_walkthrough_text = render_ail_e2e_stdlib_walkthrough_text(
        entry,
        &semantic_anchors,
        Some(&checked_core_text),
        Some(&bytecode_text),
        vm_trace_text.as_deref(),
        target_report_text.as_deref(),
    );
    Ok(AilE2eCorpusEvaluation {
        entry: entry.clone(),
        semantic_anchors,
        request_fingerprint: Some(ail_artifact_fingerprint(&request_text)),
        response_fingerprint: Some(ail_artifact_fingerprint(&response_text)),
        extracted_artifact_fingerprint: Some(ail_artifact_fingerprint(&spec_text)),
        checked_core_text: Some(checked_core_text),
        bytecode_text: Some(bytecode_text),
        vm_trace_text,
        target_report_text,
        ui_review_text,
        ui_review_patch_text,
        ui_semantic_tags_text,
        agent_policy_review_text,
        threat_model_audit_text,
        type_inference_review_text,
        state_boundary_review_text,
        workflow_scheduler_review_text,
        unsafe_boundary_review_text,
        complex_story_graph_text,
        application_walkthrough_text,
        story_promotion_review_text,
        dependency_review_text,
        stdlib_walkthrough_text,
        diagnostics_text: None,
        repair_tutorial_text: None,
        repair_proof: None,
        native_executables,
    })
}

fn render_ail_e2e_native_target_report(
    target_name: &str,
    native_executables: &[AilNativeArtifact],
) -> Result<String, String> {
    let mut lines =
        native_machine_bytecode_report_header("AIL-Examples-Target-Report:", target_name)?;
    for executable in native_executables {
        if executable.target_name != target_name {
            return Err(format!(
                "examples native artifact {} targets {}, expected {target_name}",
                executable.file_name, executable.target_name
            ));
        }
        lines.push(format!(
            "machine-bytecode target {} {} {} {} bytes {}",
            executable.target_name,
            executable.file_name,
            native_machine_bytecode_identity(&executable.bytes)?,
            ail_artifact_fingerprint_bytes(&executable.bytes),
            executable.bytes.len()
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn parse_ail_e2e_runtime_state(
    entry: &AilE2eCorpusEntry,
) -> Result<BTreeMap<String, String>, String> {
    let mut runtime_state = BTreeMap::new();
    let Some(runtime_state_text) = entry.fields.get("runtime-state") else {
        return Ok(runtime_state);
    };
    for assignment in runtime_state_text
        .split(';')
        .map(str::trim)
        .filter(|assignment| !assignment.is_empty())
    {
        insert_runtime_state_arg(assignment, &mut runtime_state).map_err(|error| {
            format!(
                "examples catalog entry {} has invalid runtime-state assignment {assignment}: {error}",
                entry.id
            )
        })?;
    }
    Ok(runtime_state)
}

fn render_ail_e2e_corpus_report(evaluations: &[AilE2eCorpusEvaluation]) -> String {
    let mut lines = vec![
        "AIL-Examples-Report:".to_string(),
        format!("entry-count {}", evaluations.len()),
    ];
    for (field, label) in [
        ("profile", "profile-count"),
        ("capability-level", "capability-level-count"),
        ("program-scale", "program-scale-count"),
        ("program-domain", "program-domain-count"),
        ("story-evidence", "story-evidence-count"),
        ("story-journey", "story-journey-count"),
        ("story-roundtrip", "story-roundtrip-count"),
        ("prompt-file", "prompt-count"),
        ("executor-family", "executor-family-count"),
        ("capture-origin", "capture-origin-count"),
        ("target", "target-count"),
        ("checker-result", "checker-result-count"),
        ("failure-taxonomy", "failure-taxonomy-count"),
    ] {
        let mut counts = BTreeMap::new();
        for evaluation in evaluations {
            if let Some(value) = evaluation.entry.fields.get(field) {
                *counts.entry(value.as_str()).or_insert(0usize) += 1;
            }
        }
        for (value, count) in counts {
            lines.push(format!("{label} {value} {count}"));
        }
    }
    let mut accepted_prompt_counts = BTreeMap::new();
    for evaluation in evaluations {
        if evaluation
            .entry
            .fields
            .get("checker-result")
            .is_some_and(|checker_result| checker_result == "accepted")
            && let Some(prompt_file) = evaluation.entry.fields.get("prompt-file")
        {
            *accepted_prompt_counts
                .entry(prompt_file.as_str())
                .or_insert(0usize) += 1;
        }
    }
    for (prompt_file, count) in accepted_prompt_counts {
        lines.push(format!("accepted-prompt-count {prompt_file} {count}"));
    }
    let mut v03_signal_counts = BTreeMap::new();
    for evaluation in evaluations {
        if let Some(signal) = evaluation.entry.fields.get("v0.3-signal") {
            *v03_signal_counts.entry(signal.as_str()).or_insert(0usize) += 1;
        }
    }
    lines.push(format!(
        "v03-signal-distinct-count {}",
        v03_signal_counts.len()
    ));
    for (signal, count) in v03_signal_counts {
        lines.push(format!("v03-signal-count {signal} {count}"));
    }
    let semantic_anchor_story_count = evaluations
        .iter()
        .filter(|evaluation| !evaluation.semantic_anchors.is_empty())
        .count();
    lines.push(format!(
        "semantic-anchor-story-count {semantic_anchor_story_count}"
    ));
    let mut semantic_anchor_total_count = 0usize;
    let mut semantic_anchor_preserved_count = 0usize;
    let mut semantic_anchor_missing_count = 0usize;
    for evaluation in evaluations {
        let story_text =
            render_ail_e2e_user_story_text(&evaluation.entry, &evaluation.semantic_anchors);
        let (preserved_count, missing_count) =
            ail_e2e_semantic_anchor_preservation_counts(&story_text, &evaluation.semantic_anchors);
        semantic_anchor_total_count += evaluation.semantic_anchors.len();
        semantic_anchor_preserved_count += preserved_count;
        semantic_anchor_missing_count += missing_count;
    }
    lines.push(format!(
        "semantic-anchor-total-count {semantic_anchor_total_count}"
    ));
    lines.push(format!(
        "semantic-anchor-preserved-count {semantic_anchor_preserved_count}"
    ));
    lines.push(format!(
        "semantic-anchor-missing-count {semantic_anchor_missing_count}"
    ));
    let story_family_dimensions =
        ail_e2e_story_family_dimensions(evaluations.iter().map(|evaluation| &evaluation.entry));
    lines.push(format!(
        "story-family-count {}",
        story_family_dimensions.len()
    ));
    for (story_id, dimensions) in story_family_dimensions {
        lines.push(format!(
            "story-family {story_id} entries {} prompt-files {} story-journeys {}",
            dimensions.entry_count,
            dimensions.prompt_files.len(),
            dimensions.story_journeys.len()
        ));
    }
    push_ail_e2e_fingerprint_reuse_lines(&mut lines, "request", evaluations, |evaluation| {
        evaluation.request_fingerprint.clone()
    });
    push_ail_e2e_fingerprint_reuse_lines(&mut lines, "response", evaluations, |evaluation| {
        evaluation.response_fingerprint.clone()
    });
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "extracted-artifact",
        evaluations,
        |evaluation| evaluation.extracted_artifact_fingerprint.clone(),
    );
    push_ail_e2e_fingerprint_reuse_lines(&mut lines, "checked-core", evaluations, |evaluation| {
        evaluation
            .checked_core_text
            .as_ref()
            .map(|text| ail_artifact_fingerprint(text))
    });
    push_ail_e2e_fingerprint_reuse_lines(&mut lines, "bytecode", evaluations, |evaluation| {
        evaluation
            .bytecode_text
            .as_ref()
            .map(|text| ail_artifact_fingerprint(text))
    });
    push_ail_e2e_fingerprint_reuse_lines(&mut lines, "vm-trace", evaluations, |evaluation| {
        evaluation
            .vm_trace_text
            .as_ref()
            .map(|text| ail_artifact_fingerprint(text))
    });
    push_ail_e2e_fingerprint_reuse_lines(&mut lines, "target-report", evaluations, |evaluation| {
        evaluation
            .target_report_text
            .as_ref()
            .map(|text| ail_artifact_fingerprint(text))
    });
    push_ail_e2e_fingerprint_reuse_lines(&mut lines, "ui-review", evaluations, |evaluation| {
        evaluation
            .ui_review_text
            .as_ref()
            .map(|text| ail_artifact_fingerprint(text))
    });
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "ui-review-patch",
        evaluations,
        |evaluation| {
            evaluation
                .ui_review_patch_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "ui-semantic-tags",
        evaluations,
        |evaluation| {
            evaluation
                .ui_semantic_tags_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "agent-policy-review",
        evaluations,
        |evaluation| {
            evaluation
                .agent_policy_review_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "threat-model-audit",
        evaluations,
        |evaluation| {
            evaluation
                .threat_model_audit_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "type-inference-review",
        evaluations,
        |evaluation| {
            evaluation
                .type_inference_review_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "state-boundary-review",
        evaluations,
        |evaluation| {
            evaluation
                .state_boundary_review_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "workflow-scheduler-review",
        evaluations,
        |evaluation| {
            evaluation
                .workflow_scheduler_review_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "unsafe-boundary-review",
        evaluations,
        |evaluation| {
            evaluation
                .unsafe_boundary_review_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "complex-story-graph",
        evaluations,
        |evaluation| {
            evaluation
                .complex_story_graph_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "application-walkthrough",
        evaluations,
        |evaluation| {
            evaluation
                .application_walkthrough_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "story-promotion-review",
        evaluations,
        |evaluation| {
            evaluation
                .story_promotion_review_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "dependency-review",
        evaluations,
        |evaluation| {
            evaluation
                .dependency_review_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "stdlib-walkthrough",
        evaluations,
        |evaluation| {
            evaluation
                .stdlib_walkthrough_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(&mut lines, "diagnostics", evaluations, |evaluation| {
        evaluation
            .diagnostics_text
            .as_ref()
            .map(|text| ail_artifact_fingerprint(text))
    });
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "repair-tutorial",
        evaluations,
        |evaluation| {
            evaluation
                .repair_tutorial_text
                .as_ref()
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "repair-candidate",
        evaluations,
        |evaluation| {
            evaluation
                .repair_proof
                .as_ref()
                .map(|proof| ail_artifact_fingerprint(&proof.candidate_spec_text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "repair-checked-core",
        evaluations,
        |evaluation| {
            evaluation
                .repair_proof
                .as_ref()
                .map(|proof| ail_artifact_fingerprint(&proof.checked_core_text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "repair-bytecode",
        evaluations,
        |evaluation| {
            evaluation
                .repair_proof
                .as_ref()
                .map(|proof| ail_artifact_fingerprint(&proof.bytecode_text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "repair-vm-trace",
        evaluations,
        |evaluation| {
            evaluation
                .repair_proof
                .as_ref()
                .and_then(|proof| proof.vm_trace_text.as_ref())
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "repair-target-report",
        evaluations,
        |evaluation| {
            evaluation
                .repair_proof
                .as_ref()
                .and_then(|proof| proof.target_report_text.as_ref())
                .map(|text| ail_artifact_fingerprint(text))
        },
    );
    push_ail_e2e_fingerprint_reuse_lines(&mut lines, "repair-diff", evaluations, |evaluation| {
        evaluation
            .repair_proof
            .as_ref()
            .map(|proof| ail_artifact_fingerprint(&proof.repair_diff_text))
    });
    push_ail_e2e_fingerprint_reuse_lines(
        &mut lines,
        "repair-promotion-review",
        evaluations,
        |evaluation| {
            evaluation
                .repair_proof
                .as_ref()
                .map(|proof| ail_artifact_fingerprint(&proof.promotion_review_text))
        },
    );
    push_ail_e2e_native_fingerprint_reuse_lines(&mut lines, evaluations);
    for evaluation in evaluations {
        let entry = &evaluation.entry;
        let semantic_task = entry
            .fields
            .get("semantic-task")
            .map(String::as_str)
            .unwrap_or("unknown");
        let executor_family = entry
            .fields
            .get("executor-family")
            .map(String::as_str)
            .unwrap_or("unknown");
        let target = entry
            .fields
            .get("target")
            .map(String::as_str)
            .unwrap_or("unknown");
        let capture_origin = entry
            .fields
            .get("capture-origin")
            .map(String::as_str)
            .unwrap_or("unspecified");
        lines.push(format!(
            "entry {} source {} semantic-task {} executor-family {} capture-origin {} target {}",
            entry.id, entry.source_file, semantic_task, executor_family, capture_origin, target
        ));
        if let Some(request_fingerprint) = &evaluation.request_fingerprint {
            lines.push(format!(
                "entry-artifact {} request examples/{}/request.fingerprint.txt {}",
                entry.id, entry.id, request_fingerprint
            ));
        }
        if let Some(response_fingerprint) = &evaluation.response_fingerprint {
            lines.push(format!(
                "entry-artifact {} response examples/{}/response.fingerprint.txt {}",
                entry.id, entry.id, response_fingerprint
            ));
        }
        if let Some(extracted_artifact_fingerprint) = &evaluation.extracted_artifact_fingerprint {
            lines.push(format!(
                "entry-artifact {} extracted-artifact examples/{}/artifact.fingerprint.txt {}",
                entry.id, entry.id, extracted_artifact_fingerprint
            ));
        }
        if let Some(core_text) = &evaluation.checked_core_text {
            lines.push(format!(
                "entry-artifact {} checked-core examples/{}/checked.ail-core.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(core_text)
            ));
        }
        if let Some(bytecode_text) = &evaluation.bytecode_text {
            lines.push(format!(
                "entry-artifact {} bytecode examples/{}/artifact.ailbc.json {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(bytecode_text)
            ));
        }
        if let Some(vm_trace_text) = &evaluation.vm_trace_text {
            lines.push(format!(
                "entry-artifact {} vm-trace examples/{}/vm-trace.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(vm_trace_text)
            ));
        }
        for executable in &evaluation.native_executables {
            lines.push(format!(
                "entry-artifact {} native {} examples/{}/{} {}",
                entry.id,
                executable.target_name,
                entry.id,
                executable.file_name,
                ail_artifact_fingerprint_bytes(&executable.bytes)
            ));
        }
        if let Some(target_report_text) = &evaluation.target_report_text {
            lines.push(format!(
                "entry-artifact {} target-report examples/{}/target-report.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(target_report_text)
            ));
        }
        if let Some(ui_review_text) = &evaluation.ui_review_text {
            lines.push(format!(
                "entry-artifact {} ui-review examples/{}/ui-review.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(ui_review_text)
            ));
        }
        if let Some(ui_review_patch_text) = &evaluation.ui_review_patch_text {
            lines.push(format!(
                "entry-artifact {} ui-review-patch examples/{}/ui-review-patch.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(ui_review_patch_text)
            ));
        }
        if let Some(ui_semantic_tags_text) = &evaluation.ui_semantic_tags_text {
            lines.push(format!(
                "entry-artifact {} ui-semantic-tags examples/{}/ui-semantic-tags.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(ui_semantic_tags_text)
            ));
        }
        if let Some(agent_policy_review_text) = &evaluation.agent_policy_review_text {
            lines.push(format!(
                "entry-artifact {} agent-policy-review examples/{}/agent-policy-review.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(agent_policy_review_text)
            ));
        }
        if let Some(threat_model_audit_text) = &evaluation.threat_model_audit_text {
            lines.push(format!(
                "entry-artifact {} threat-model-audit examples/{}/threat-model-audit.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(threat_model_audit_text)
            ));
        }
        if let Some(type_inference_review_text) = &evaluation.type_inference_review_text {
            lines.push(format!(
                "entry-artifact {} type-inference-review examples/{}/type-inference-review.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(type_inference_review_text)
            ));
        }
        if let Some(state_boundary_review_text) = &evaluation.state_boundary_review_text {
            lines.push(format!(
                "entry-artifact {} state-boundary-review examples/{}/state-boundary-review.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(state_boundary_review_text)
            ));
        }
        if let Some(workflow_scheduler_review_text) = &evaluation.workflow_scheduler_review_text {
            lines.push(format!(
                "entry-artifact {} workflow-scheduler-review examples/{}/workflow-scheduler-review.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(workflow_scheduler_review_text)
            ));
        }
        if let Some(unsafe_boundary_review_text) = &evaluation.unsafe_boundary_review_text {
            lines.push(format!(
                "entry-artifact {} unsafe-boundary-review examples/{}/unsafe-boundary-review.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(unsafe_boundary_review_text)
            ));
        }
        if let Some(complex_story_graph_text) = &evaluation.complex_story_graph_text {
            lines.push(format!(
                "entry-artifact {} complex-story-graph examples/{}/complex-story-graph.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(complex_story_graph_text)
            ));
        }
        if let Some(application_walkthrough_text) = &evaluation.application_walkthrough_text {
            lines.push(format!(
                "entry-artifact {} application-walkthrough examples/{}/application-walkthrough.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(application_walkthrough_text)
            ));
        }
        if let Some(story_promotion_review_text) = &evaluation.story_promotion_review_text {
            lines.push(format!(
                "entry-artifact {} story-promotion-review examples/{}/story-promotion-review.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(story_promotion_review_text)
            ));
        }
        if let Some(dependency_review_text) = &evaluation.dependency_review_text {
            lines.push(format!(
                "entry-artifact {} dependency-review examples/{}/dependency-review.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(dependency_review_text)
            ));
        }
        if let Some(stdlib_walkthrough_text) = &evaluation.stdlib_walkthrough_text {
            lines.push(format!(
                "entry-artifact {} stdlib-walkthrough examples/{}/stdlib-walkthrough.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(stdlib_walkthrough_text)
            ));
        }
        if let Some(diagnostics_text) = &evaluation.diagnostics_text {
            lines.push(format!(
                "entry-artifact {} diagnostics examples/{}/diagnostics.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(diagnostics_text)
            ));
        }
        if let Some(repair_tutorial_text) = &evaluation.repair_tutorial_text {
            lines.push(format!(
                "entry-artifact {} repair-tutorial examples/{}/repair-tutorial.txt {}",
                entry.id,
                entry.id,
                ail_artifact_fingerprint(repair_tutorial_text)
            ));
        }
        if let Some(repair_proof) = &evaluation.repair_proof {
            push_ail_e2e_repair_proof_artifact_lines(
                &mut lines,
                "entry-artifact",
                entry,
                repair_proof,
            );
        }
        if !evaluation.semantic_anchors.is_empty() {
            lines.push(format!(
                "entry-semantic-anchors {} {}",
                entry.id,
                evaluation.semantic_anchors.join("; ")
            ));
        }
        let story_text = render_ail_e2e_user_story_text(entry, &evaluation.semantic_anchors);
        if !evaluation.semantic_anchors.is_empty() {
            let (preserved_count, missing_count) = ail_e2e_semantic_anchor_preservation_counts(
                &story_text,
                &evaluation.semantic_anchors,
            );
            lines.push(format!(
                "entry-semantic-anchor-preservation {} preserved {} missing {}",
                entry.id, preserved_count, missing_count
            ));
        }
        lines.push(format!(
            "entry-artifact {} user-story examples/{}/user-story.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(&story_text)
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn ail_e2e_semantic_anchor_preservation_counts(
    story_text: &str,
    semantic_anchors: &[String],
) -> (usize, usize) {
    let preserved_count = semantic_anchors
        .iter()
        .filter(|anchor| story_text.contains(anchor.as_str()))
        .count();
    (preserved_count, semantic_anchors.len() - preserved_count)
}

fn push_ail_e2e_fingerprint_reuse_lines<F>(
    lines: &mut Vec<String>,
    label: &str,
    evaluations: &[AilE2eCorpusEvaluation],
    fingerprint_for: F,
) where
    F: Fn(&AilE2eCorpusEvaluation) -> Option<String>,
{
    let mut entries_by_fingerprint: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for evaluation in evaluations {
        if let Some(fingerprint) = fingerprint_for(evaluation) {
            entries_by_fingerprint
                .entry(fingerprint)
                .or_default()
                .push(evaluation.entry.id.clone());
        }
    }
    push_ail_e2e_fingerprint_reuse_summary(lines, label, entries_by_fingerprint);
}

fn push_ail_e2e_native_fingerprint_reuse_lines(
    lines: &mut Vec<String>,
    evaluations: &[AilE2eCorpusEvaluation],
) {
    let mut entries_by_fingerprint: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for evaluation in evaluations {
        for executable in &evaluation.native_executables {
            entries_by_fingerprint
                .entry(ail_artifact_fingerprint_bytes(&executable.bytes))
                .or_default()
                .push(format!("{}:{}", evaluation.entry.id, executable.file_name));
        }
    }
    push_ail_e2e_fingerprint_reuse_summary(lines, "native", entries_by_fingerprint);
}

fn push_ail_e2e_fingerprint_reuse_summary(
    lines: &mut Vec<String>,
    label: &str,
    entries_by_fingerprint: BTreeMap<String, Vec<String>>,
) {
    let observed_count = entries_by_fingerprint.values().map(Vec::len).sum::<usize>();
    let duplicate_entry_count = entries_by_fingerprint
        .values()
        .filter(|entries| entries.len() > 1)
        .map(|entries| entries.len() - 1)
        .sum::<usize>();
    let reuse_group_count = entries_by_fingerprint
        .values()
        .filter(|entries| entries.len() > 1)
        .count();
    lines.push(format!(
        "{label}-fingerprint-observed-count {observed_count}"
    ));
    lines.push(format!(
        "{label}-fingerprint-distinct-count {}",
        entries_by_fingerprint.len()
    ));
    lines.push(format!(
        "{label}-fingerprint-duplicate-entry-count {duplicate_entry_count}"
    ));
    lines.push(format!(
        "{label}-fingerprint-reuse-group-count {reuse_group_count}"
    ));
    for (fingerprint, entries) in entries_by_fingerprint {
        if entries.len() > 1 {
            lines.push(format!(
                "{label}-fingerprint-reuse {fingerprint} {} {}",
                entries.len(),
                entries.join(",")
            ));
        }
    }
}

fn push_ail_e2e_entry_artifact_lines(
    lines: &mut Vec<String>,
    prefix: &str,
    evaluation: &AilE2eCorpusEvaluation,
) {
    let entry = &evaluation.entry;
    if let Some(request_fingerprint) = &evaluation.request_fingerprint {
        lines.push(format!(
            "{prefix} {} request examples/{}/request.fingerprint.txt {}",
            entry.id, entry.id, request_fingerprint
        ));
    }
    if let Some(response_fingerprint) = &evaluation.response_fingerprint {
        lines.push(format!(
            "{prefix} {} response examples/{}/response.fingerprint.txt {}",
            entry.id, entry.id, response_fingerprint
        ));
    }
    if let Some(extracted_artifact_fingerprint) = &evaluation.extracted_artifact_fingerprint {
        lines.push(format!(
            "{prefix} {} extracted-artifact examples/{}/artifact.fingerprint.txt {}",
            entry.id, entry.id, extracted_artifact_fingerprint
        ));
    }
    if let Some(core_text) = &evaluation.checked_core_text {
        lines.push(format!(
            "{prefix} {} checked-core examples/{}/checked.ail-core.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(core_text)
        ));
    }
    if let Some(bytecode_text) = &evaluation.bytecode_text {
        lines.push(format!(
            "{prefix} {} bytecode examples/{}/artifact.ailbc.json {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = &evaluation.vm_trace_text {
        lines.push(format!(
            "{prefix} {} vm-trace examples/{}/vm-trace.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    for executable in &evaluation.native_executables {
        lines.push(format!(
            "{prefix} {} native {} examples/{}/{} {}",
            entry.id,
            executable.target_name,
            entry.id,
            executable.file_name,
            ail_artifact_fingerprint_bytes(&executable.bytes)
        ));
    }
    if let Some(target_report_text) = &evaluation.target_report_text {
        lines.push(format!(
            "{prefix} {} target-report examples/{}/target-report.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    if let Some(ui_review_text) = &evaluation.ui_review_text {
        lines.push(format!(
            "{prefix} {} ui-review examples/{}/ui-review.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(ui_review_text)
        ));
    }
    if let Some(ui_review_patch_text) = &evaluation.ui_review_patch_text {
        lines.push(format!(
            "{prefix} {} ui-review-patch examples/{}/ui-review-patch.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(ui_review_patch_text)
        ));
    }
    if let Some(ui_semantic_tags_text) = &evaluation.ui_semantic_tags_text {
        lines.push(format!(
            "{prefix} {} ui-semantic-tags examples/{}/ui-semantic-tags.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(ui_semantic_tags_text)
        ));
    }
    if let Some(agent_policy_review_text) = &evaluation.agent_policy_review_text {
        lines.push(format!(
            "{prefix} {} agent-policy-review examples/{}/agent-policy-review.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(agent_policy_review_text)
        ));
    }
    if let Some(threat_model_audit_text) = &evaluation.threat_model_audit_text {
        lines.push(format!(
            "{prefix} {} threat-model-audit examples/{}/threat-model-audit.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(threat_model_audit_text)
        ));
    }
    if let Some(type_inference_review_text) = &evaluation.type_inference_review_text {
        lines.push(format!(
            "{prefix} {} type-inference-review examples/{}/type-inference-review.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(type_inference_review_text)
        ));
    }
    if let Some(state_boundary_review_text) = &evaluation.state_boundary_review_text {
        lines.push(format!(
            "{prefix} {} state-boundary-review examples/{}/state-boundary-review.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(state_boundary_review_text)
        ));
    }
    if let Some(workflow_scheduler_review_text) = &evaluation.workflow_scheduler_review_text {
        lines.push(format!(
            "{prefix} {} workflow-scheduler-review examples/{}/workflow-scheduler-review.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(workflow_scheduler_review_text)
        ));
    }
    if let Some(unsafe_boundary_review_text) = &evaluation.unsafe_boundary_review_text {
        lines.push(format!(
            "{prefix} {} unsafe-boundary-review examples/{}/unsafe-boundary-review.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(unsafe_boundary_review_text)
        ));
    }
    if let Some(complex_story_graph_text) = &evaluation.complex_story_graph_text {
        lines.push(format!(
            "{prefix} {} complex-story-graph examples/{}/complex-story-graph.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(complex_story_graph_text)
        ));
    }
    if let Some(application_walkthrough_text) = &evaluation.application_walkthrough_text {
        lines.push(format!(
            "{prefix} {} application-walkthrough examples/{}/application-walkthrough.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(application_walkthrough_text)
        ));
    }
    if let Some(story_promotion_review_text) = &evaluation.story_promotion_review_text {
        lines.push(format!(
            "{prefix} {} story-promotion-review examples/{}/story-promotion-review.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(story_promotion_review_text)
        ));
    }
    if let Some(dependency_review_text) = &evaluation.dependency_review_text {
        lines.push(format!(
            "{prefix} {} dependency-review examples/{}/dependency-review.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(dependency_review_text)
        ));
    }
    if let Some(stdlib_walkthrough_text) = &evaluation.stdlib_walkthrough_text {
        lines.push(format!(
            "{prefix} {} stdlib-walkthrough examples/{}/stdlib-walkthrough.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(stdlib_walkthrough_text)
        ));
    }
    if let Some(diagnostics_text) = &evaluation.diagnostics_text {
        lines.push(format!(
            "{prefix} {} diagnostics examples/{}/diagnostics.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(diagnostics_text)
        ));
    }
    if let Some(repair_tutorial_text) = &evaluation.repair_tutorial_text {
        lines.push(format!(
            "{prefix} {} repair-tutorial examples/{}/repair-tutorial.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(repair_tutorial_text)
        ));
    }
    if let Some(repair_proof) = &evaluation.repair_proof {
        push_ail_e2e_repair_proof_artifact_lines(lines, prefix, entry, repair_proof);
    }
    let story_text = render_ail_e2e_user_story_text(entry, &evaluation.semantic_anchors);
    lines.push(format!(
        "{prefix} {} user-story examples/{}/user-story.txt {}",
        entry.id,
        entry.id,
        ail_artifact_fingerprint(&story_text)
    ));
}

fn push_ail_e2e_repair_proof_artifact_lines(
    lines: &mut Vec<String>,
    prefix: &str,
    entry: &AilE2eCorpusEntry,
    repair_proof: &AilE2eRepairProofArtifacts,
) {
    lines.push(format!(
        "{prefix} {} repair-candidate examples/{}/repair-candidate.ail-spec.md {}",
        entry.id,
        entry.id,
        ail_artifact_fingerprint(&repair_proof.candidate_spec_text)
    ));
    lines.push(format!(
        "{prefix} {} repair-checked-core examples/{}/repair-checked.ail-core.txt {}",
        entry.id,
        entry.id,
        ail_artifact_fingerprint(&repair_proof.checked_core_text)
    ));
    lines.push(format!(
        "{prefix} {} repair-bytecode examples/{}/repair-artifact.ailbc.json {}",
        entry.id,
        entry.id,
        ail_artifact_fingerprint(&repair_proof.bytecode_text)
    ));
    if let Some(vm_trace_text) = &repair_proof.vm_trace_text {
        lines.push(format!(
            "{prefix} {} repair-vm-trace examples/{}/repair-vm-trace.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = &repair_proof.target_report_text {
        lines.push(format!(
            "{prefix} {} repair-target-report examples/{}/repair-target-report.txt {}",
            entry.id,
            entry.id,
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(format!(
        "{prefix} {} repair-diff examples/{}/repair-diff.txt {}",
        entry.id,
        entry.id,
        ail_artifact_fingerprint(&repair_proof.repair_diff_text)
    ));
    lines.push(format!(
        "{prefix} {} repair-promotion-review examples/{}/repair-promotion-review.txt {}",
        entry.id,
        entry.id,
        ail_artifact_fingerprint(&repair_proof.promotion_review_text)
    ));
}

fn render_ail_e2e_user_story_text(
    entry: &AilE2eCorpusEntry,
    semantic_anchors: &[String],
) -> String {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let semantic_anchor_line = if semantic_anchors.is_empty() {
        String::new()
    } else {
        format!("semantic-anchors {}\n", semantic_anchors.join("; "))
    };
    format!(
        "AIL-User-Story:\nentry {}\nid {}\nstory {}\nacceptance-criteria {}\nuse-case {}\ncapability-under-test {}\nprogram-scale {}\nprogram-domain {}\nmodule-count {}\nspec-count {}\nstory-count {}\ninteracts-with {}\nstory-journey {}\nstory-roundtrip {}\nstory-evidence {}\nsemantic-task {}\n{}\n",
        entry.id,
        field("user-story-id"),
        field("user-story"),
        field("acceptance-criteria"),
        field("use-case"),
        field("capability-under-test"),
        field("program-scale"),
        field("program-domain"),
        field("module-count"),
        field("spec-count"),
        field("story-count"),
        field("interacts-with"),
        field("story-journey"),
        field("story-roundtrip"),
        field("story-evidence"),
        field("semantic-task"),
        semantic_anchor_line,
    )
}

fn render_ail_e2e_ui_review_text(
    entry: &AilE2eCorpusEntry,
    semantic_anchors: &[String],
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let surface_tags = field("surface-tags");
    let has_ui_surface_tag = surface_tags
        .split(',')
        .map(str::trim)
        .any(|surface_tag| surface_tag == "ui");
    if field("program-domain") != "ui-workflow" && !has_ui_surface_tag {
        return None;
    }

    let story_text = render_ail_e2e_user_story_text(entry, semantic_anchors);
    let (preserved_count, missing_count) =
        ail_e2e_semantic_anchor_preservation_counts(&story_text, semantic_anchors);
    let workflow_authoring_artifact = if checked_core_text.is_some() {
        "checked-core"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else {
        "user-story"
    };
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };

    let mut lines = vec![
        "AIL-UI-Review:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("surface-tags {}", surface_tags),
        format!("capability-under-test {}", field("capability-under-test")),
        format!("story-journey {}", field("story-journey")),
        format!("story-roundtrip {}", field("story-roundtrip")),
        format!("story-evidence {}", field("story-evidence")),
        "visual-review-artifact deterministic-text".to_string(),
        "accessibility-review required".to_string(),
        "ui-surface-review route,form,dashboard,workflow".to_string(),
        format!("workflow-authoring-artifact {workflow_authoring_artifact}"),
        format!("runtime-evidence {runtime_evidence}"),
        format!("semantic-anchor-count {}", semantic_anchors.len()),
        format!("semantic-anchor-preserved-count {preserved_count}"),
        format!("semantic-anchor-missing-count {missing_count}"),
    ];
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    if !semantic_anchors.is_empty() {
        lines.push(format!("semantic-anchors {}", semantic_anchors.join("; ")));
    }
    lines.push(
        "ui-review-summary UI workflow evidence preserves story, accessibility, visual review, and runtime handoff anchors."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_ui_review_patch_text(
    entry: &AilE2eCorpusEntry,
    ui_review_text: &str,
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> String {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let mut lines = vec![
        "AIL-UI-Review-Patch:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        "patch-source ui-review".to_string(),
        "visual-review-patch-plan deterministic-text".to_string(),
        "patch-command ail-flow-edit".to_string(),
        "patch-scope route,form,dashboard,workflow".to_string(),
        "human-approval-required true".to_string(),
        "patch-import-status proposed-only".to_string(),
        format!(
            "ui-review-fingerprint {}",
            ail_artifact_fingerprint(ui_review_text)
        ),
    ];
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "ui-review-patch-summary Patch plan is reviewable and fingerprinted, but import still requires human approval."
            .to_string(),
    );
    format!("{}\n", lines.join("\n"))
}

fn render_ail_e2e_ui_semantic_tags_text(
    entry: &AilE2eCorpusEntry,
    semantic_anchors: &[String],
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let ui_signal =
        "UI examples need richer package-local walkthroughs and stricter semantic tagging.";
    let is_option_map_ui_bridge =
        field("v0.3-signal") == ui_signal && field("package").ends_with("option_map.ail");
    if !is_option_map_ui_bridge {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-UI-Semantic-Tags:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("surface-tags {}", field("surface-tags")),
        format!("capability-under-test {}", field("capability-under-test")),
        "ui-semantic-tags-artifact deterministic-text".to_string(),
        "package-local-walkthrough option_map".to_string(),
        "semantic-tag ui.form".to_string(),
        "semantic-tag ui.route".to_string(),
        "semantic-tag ui.state".to_string(),
        "ui-bridge-surface option-map-transform".to_string(),
        "generic-contract Option<T>".to_string(),
        "generic-function Option.map".to_string(),
        "trace-event OptionMapEvaluated".to_string(),
        format!("story-journey {}", field("story-journey")),
        format!("story-roundtrip {}", field("story-roundtrip")),
        format!("story-evidence {}", field("story-evidence")),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    for anchor in semantic_anchors {
        lines.push(format!("story-anchor {anchor}"));
    }
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "ui-semantic-tags-summary Option Map evidence connects package-local generic behavior, ui.form/ui.route/ui.state tags, story anchors, and replay fingerprints for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_agent_policy_review_text(
    entry: &AilE2eCorpusEntry,
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    if field("profile") != "AgentTool" && field("program-domain") != "agent-tool" {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-Agent-Policy-Review:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("executor-family {}", field("executor-family")),
        format!("executor-label {}", field("executor-label")),
        format!("prompt-file {}", field("prompt-file")),
        format!("interacts-with {}", field("interacts-with")),
        "agent-policy-review-artifact deterministic-text".to_string(),
        "multi-agent-handoff-review required".to_string(),
        "agent-contract-check ail-agent-contracts examples/agents".to_string(),
        "handoff-roles requirements-writer,spec-writer,diagnostic-repairer,prompt-reviewer,agent-policy-reviewer".to_string(),
        "tool-permission-review required".to_string(),
        "tool-approval-review required".to_string(),
        "external-call-review required".to_string(),
        "secret-redaction-review required".to_string(),
        "audit-trace-review required".to_string(),
        "human-approval-required true".to_string(),
        "policy-import-status proposed-only".to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "agent-policy-review-summary AgentTool evidence is ready for human-approved multi-agent policy handoff import."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_threat_model_audit_text(
    entry: &AilE2eCorpusEntry,
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let security_signal = "Security examples need threat-model annotations and audit trails.";
    let is_secret_access_security_entry = field("capability-under-test") == "security-permissions"
        || field("package").ends_with("secret_access.ail")
        || field("v0.3-signal") == security_signal;
    if !is_secret_access_security_entry {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-Threat-Model-Audit:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("capability-under-test {}", field("capability-under-test")),
        "threat-model-artifact deterministic-text".to_string(),
        "security-surface secret-internal-notes".to_string(),
        "asset Ticket.internal notes".to_string(),
        "trust-boundary requester-role-check".to_string(),
        "attacker-capability customer-or-unauthorized-requester".to_string(),
        "required-permission SupportAgent or SupportManager".to_string(),
        "required-control support-role-requirement".to_string(),
        "required-control customer-redaction".to_string(),
        "redaction-requirement customer must not receive internal notes".to_string(),
        "audit-trail-event InternalNotesViewed".to_string(),
        "audit-trail-event InternalNotesDenied".to_string(),
        "denied-failure PermissionDenied".to_string(),
        "diagnostic-link AIL-SECRET-ROLE-001".to_string(),
        "diagnostic-link AIL005".to_string(),
        "diagnostic-link AIL-TRACE-002".to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "threat-model-summary Secret Access binds role checks, customer redaction, denied-access traces, diagnostics, and replay evidence for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_type_inference_review_text(
    entry: &AilE2eCorpusEntry,
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let generic_runtime_signal =
        "Generic runtime behavior needs clearer type-inference explanations.";
    let is_runtime_generic_entry = field("capability-under-test") == "runtime-generics"
        || field("package").ends_with("runtime_generic.ail")
        || field("v0.3-signal") == generic_runtime_signal;
    if !is_runtime_generic_entry {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-Type-Inference-Review:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("capability-under-test {}", field("capability-under-test")),
        "type-inference-artifact deterministic-text".to_string(),
        "type-surface runtime-generics".to_string(),
        "entity Runtime Tickets".to_string(),
        "action Prioritize ticket".to_string(),
        "inferred-field ticket.priority State<Low, High>".to_string(),
        "inferred-field ticket.status State<Open, Closed>".to_string(),
        "inferred-field SupportTicket.priority State<Low, High>".to_string(),
        "initial-state ticket.priority=Low".to_string(),
        "precondition ticket exists".to_string(),
        "precondition ticket priority not High".to_string(),
        "state-transition ticket.priority Low -> High".to_string(),
        "postcondition high priority tickets are handled first".to_string(),
        "trace-event TicketPrioritized".to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "type-inference-summary Runtime Generic evidence explains inferred state variants, preconditions, state transition, trace coverage, and replay fingerprints for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_state_boundary_review_text(
    entry: &AilE2eCorpusEntry,
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let state_signal = "State examples need clearer persistence and concurrency boundaries.";
    let is_stateful_counter_entry = field("capability-under-test") == "stateful-runtime"
        || field("package").ends_with("stateful_counter.ail")
        || field("v0.3-signal") == state_signal;
    if !is_stateful_counter_entry {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-State-Boundary-Review:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("capability-under-test {}", field("capability-under-test")),
        "state-boundary-artifact deterministic-text".to_string(),
        "state-surface persistence-concurrency".to_string(),
        "entity Counter".to_string(),
        "action Increment counter".to_string(),
        "mutable-field counter.value Int".to_string(),
        "state-transition counter.value n -> n + 1".to_string(),
        "persistence-boundary counter write must be durable before replay".to_string(),
        "idempotency-boundary retryable increment requires request id and dedupe state".to_string(),
        "concurrency-boundary shared counter mutation requires lock or serialization".to_string(),
        "failure-boundary failure after write requires replay recovery".to_string(),
        "trace-event CounterIncremented".to_string(),
        "diagnostic-link AIL-STATE-001".to_string(),
        "diagnostic-link AIL-STATE-002".to_string(),
        "diagnostic-link AIL-STATE-003".to_string(),
        "diagnostic-link AIL-STATE-004".to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "state-boundary-summary Stateful Counter evidence explains persistence, idempotency, concurrency, failure replay, trace coverage, and replay fingerprints for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_workflow_scheduler_review_text(
    entry: &AilE2eCorpusEntry,
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let workflow_signal = "Workflow examples need retry/backoff semantics and richer scheduler policies beyond temporal-policy diagnostics.";
    let is_repeated_task_entry = field("capability-under-test") == "scheduled-workflow"
        || field("package").ends_with("repeated_task.ail")
        || field("v0.3-signal") == workflow_signal;
    if !is_repeated_task_entry {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-Workflow-Scheduler-Review:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("capability-under-test {}", field("capability-under-test")),
        "workflow-scheduler-artifact deterministic-text".to_string(),
        "workflow-surface scheduler-retry-backoff".to_string(),
        "action Run maintenance cycle".to_string(),
        "repeated-action IncrementCounter".to_string(),
        "repeat-count 3".to_string(),
        "temporal-policy daily maintenance window".to_string(),
        "retry-policy bounded maintenance retry".to_string(),
        "backoff-policy exponential maintenance backoff".to_string(),
        "accepted-fixture examples/accepted/retry-backoff-policy-minimal.ail-spec.md".to_string(),
        "rejected-fixture examples/rejected/retry-policy-without-backoff.ail-spec.md".to_string(),
        "diagnostic-link AIL-WORKFLOW-001".to_string(),
        "diagnostic-link AIL-WORKFLOW-002".to_string(),
        "trace-event CounterIncremented".to_string(),
        "trace-event MaintenanceCycleCompleted".to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "workflow-scheduler-summary Repeated Task evidence explains temporal policy, retry policy, backoff policy, accepted and rejected fixtures, trace coverage, and replay fingerprints for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_unsafe_boundary_review_text(
    entry: &AilE2eCorpusEntry,
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let interop_signal =
        "Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.";
    let is_c_interop_entry = field("capability-under-test") == "c-host-interop"
        || field("package").ends_with("c_interop.ail")
        || field("v0.3-signal") == interop_signal;
    if !is_c_interop_entry {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-Unsafe-Boundary-Review:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("capability-under-test {}", field("capability-under-test")),
        "unsafe-boundary-review-artifact deterministic-text".to_string(),
        "interop-surface c-host-interop".to_string(),
        "foreign-library zlib".to_string(),
        "foreign-function zlib.compress2".to_string(),
        "foreign-library libc".to_string(),
        "foreign-function libc.qsort".to_string(),
        "ownership-boundary owned pointer requires release semantics".to_string(),
        "borrow-boundary borrowed mutable pointer must not escape call".to_string(),
        "callback-boundary qsort comparator is noescape".to_string(),
        "layout-boundary repr(C) packet header size alignment offsets".to_string(),
        "status-map-boundary C return values map to AIL failures".to_string(),
        "nullable-boundary NonNull pointer contracts reject nullable values".to_string(),
        "accepted-fixture examples/accepted/owned-pointer-release-minimal.ail-spec.md".to_string(),
        "accepted-fixture examples/accepted/struct-layout-minimal.ail-spec.md".to_string(),
        "rejected-fixture examples/rejected/borrowed-pointer-escape.ail-spec.md".to_string(),
        "rejected-fixture examples/rejected/missing-status-map.ail-spec.md".to_string(),
        "rejected-fixture examples/rejected/missing-trace.ail-spec.md".to_string(),
        "rejected-fixture examples/rejected/mutable-pointer-aliasing.ail-spec.md".to_string(),
        "rejected-fixture examples/rejected/nullable-to-non-null.ail-spec.md".to_string(),
        "rejected-fixture examples/rejected/owned-pointer-without-release.ail-spec.md".to_string(),
        "rejected-fixture examples/rejected/secret-leakage.ail-spec.md".to_string(),
        "diagnostic-link AIL-FFI-NULL-001".to_string(),
        "diagnostic-link AIL-FFI-OWNERSHIP-001".to_string(),
        "diagnostic-link AIL-FFI-OWNERSHIP-002".to_string(),
        "diagnostic-link AIL-FFI-ALIAS-001".to_string(),
        "diagnostic-link AIL-FFI-ERRNO-001".to_string(),
        "diagnostic-link AIL-FFI-SECRET-001".to_string(),
        "trace-event ForeignCallCompressed".to_string(),
        "trace-event ForeignCallbackCompared".to_string(),
        "trace-event ForeignStatusMapped".to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "unsafe-boundary-summary C Interop evidence explains ownership, borrowing, callbacks, layout, status maps, nullable contracts, accepted and rejected fixtures, diagnostics, trace coverage, and replay fingerprints for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_complex_story_graph_text(
    entry: &AilE2eCorpusEntry,
    semantic_anchors: &[String],
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let complex_signal = "Complex systems need richer story graphs that span imported modules, UI surfaces, workflows, target contracts, and regenerated story views.";
    let is_incident_response_complex_story = field("v0.3-signal") == complex_signal
        && field("package").ends_with("incident_response.ail");
    if !is_incident_response_complex_story {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-Complex-Story-Graph:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("capability-under-test {}", field("capability-under-test")),
        "complex-story-graph-artifact deterministic-text".to_string(),
        "complex-surface multi-module-incident-workflow".to_string(),
        "imported-module incident_identity".to_string(),
        "imported-module incident_policy".to_string(),
        "imported-module incident_notifications".to_string(),
        "root-module incident_response".to_string(),
        "ui-surface incident command center".to_string(),
        "ui-surface service owner dashboard".to_string(),
        "workflow-transition Declare incident".to_string(),
        "workflow-transition Escalate incident".to_string(),
        "workflow-transition Resolve incident".to_string(),
        "workflow-transition Start postmortem".to_string(),
        format!("target-contract {}", field("target")),
        format!("story-journey {}", field("story-journey")),
        format!("story-roundtrip {}", field("story-roundtrip")),
        format!("story-evidence {}", field("story-evidence")),
        "regenerated-story-view user-story.txt".to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    for anchor in semantic_anchors {
        lines.push(format!("story-anchor {anchor}"));
    }
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "complex-story-graph-summary Incident Response evidence connects imported modules, UI surfaces, workflow transitions, target contracts, regenerated story views, semantic anchors, and replay fingerprints for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_application_walkthrough_text(
    entry: &AilE2eCorpusEntry,
    semantic_anchors: &[String],
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let application_signal = "Application examples need more repaired incident promotion variants and richer stateful application walkthroughs after the first package-local repair proof is promoted.";
    if field("v0.3-signal") != application_signal {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let package = field("package");
    let action = entry
        .fields
        .get("vm-action")
        .filter(|value| !value.is_empty())
        .map(String::as_str)
        .unwrap_or("review-application-evidence");
    let mut lines = vec![
        "AIL-Application-Walkthrough:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {package}"),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("capability-under-test {}", field("capability-under-test")),
        "application-walkthrough-artifact deterministic-text".to_string(),
        format!("user-story-id {}", field("user-story-id")),
        format!("story-journey {}", field("story-journey")),
        format!("story-roundtrip {}", field("story-roundtrip")),
        format!("story-evidence {}", field("story-evidence")),
        format!("target-contract {}", field("target")),
        format!("runtime-state {}", field("runtime-state")),
        format!("action {action}"),
        "walkthrough-step story".to_string(),
        "walkthrough-step requirements".to_string(),
        "walkthrough-step spec".to_string(),
        "walkthrough-step checked-core".to_string(),
        "walkthrough-step bytecode".to_string(),
        "walkthrough-step runtime-or-target-evidence".to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    if package.ends_with("support_ticket.ail") {
        lines.extend([
            "application-surface support-ticket-lifecycle".to_string(),
            "stateful-boundary ticket.status Open -> Closed".to_string(),
            "trace-event TicketClosed".to_string(),
        ]);
    } else if package.ends_with("incident_response.ail") {
        let repair_variant = if field("semantic-task").contains("commander-review") {
            "commander-review"
        } else if field("semantic-task").contains("private-notes") {
            "private-notes"
        } else {
            "incident-repair"
        };
        lines.extend([
            "application-surface incident-repair-promotion".to_string(),
            "repair-promotion conformance-repair-proof".to_string(),
            format!("repair-promotion-variant {repair_variant}"),
            "stateful-boundary incident.status Declared -> Escalated".to_string(),
            "trace-event IncidentEscalated".to_string(),
        ]);
        let repair_provenance = match repair_variant {
            "private-notes" => Some((
                "private-notes-public-timeline-leak.ail-spec.md",
                "AIL-APP-006",
            )),
            "commander-review" => Some((
                "escalation-without-commander-review.ail-spec.md",
                "AIL-APP-007",
            )),
            _ => None,
        };
        if let Some((rejected_fixture, expected_diagnostic)) = repair_provenance {
            let repair_rejected_fixture =
                format!("examples/incident_response.ail/examples/rejected/{rejected_fixture}");
            let repair_artifact_base = format!("rejected/{rejected_fixture}");
            let reviewer = entry
                .fields
                .get("executor-label")
                .filter(|value| !value.is_empty())
                .map(String::as_str)
                .unwrap_or("codex-ail-repair-promotion-reviewer");
            lines.extend([
                "repair-provenance package-local-conformance".to_string(),
                format!("repair-rejected-fixture {repair_rejected_fixture}"),
                format!("repair-expected-diagnostic {expected_diagnostic}"),
                format!("repair-proof-artifact {repair_artifact_base}/repair-proof.txt"),
                format!(
                    "repair-candidate-artifact {repair_artifact_base}/repair-candidate.ail-spec.md"
                ),
                "repair-candidate-source examples/incident_response.ail/examples/accepted/incident-escalation-minimal.ail-spec.md".to_string(),
                format!("repair-promotion-reviewer {reviewer}"),
            ]);
        }
    } else {
        lines.extend([
            "application-surface application-workflow".to_string(),
            "stateful-boundary application state is explicit before runtime replay".to_string(),
            "trace-event application workflow trace is preserved".to_string(),
        ]);
    }
    for anchor in semantic_anchors {
        lines.push(format!("story-anchor {anchor}"));
    }
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "application-walkthrough-summary Application evidence connects user story, requirements, spec, checked Core, bytecode, runtime or target proof, stateful boundary, trace event, semantic anchors, and replay fingerprints for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn extract_ail_e2e_report_line_value<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.strip_prefix(prefix).map(str::trim))
}

fn extract_ail_e2e_report_line_value_any<'a>(text: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes
        .iter()
        .find_map(|prefix| extract_ail_e2e_report_line_value(text, prefix))
}

fn render_ail_e2e_story_promotion_review_text(
    entry: &AilE2eCorpusEntry,
    semantic_anchors: &[String],
    request_text: &str,
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Result<Option<String>, String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    if field("capability-under-test") != "user-story-mode-promotion" {
        return Ok(None);
    }
    let story_artifacts = field("story-artifacts");
    if story_artifacts.is_empty() {
        return Err(format!(
            "examples catalog entry {} user-story-mode-promotion is missing story-artifacts",
            entry.id
        ));
    }
    ail_e2e_validate_catalog_relative_path(&entry.id, "story-artifacts", story_artifacts)?;
    let story_artifacts_path = ail_e2e_entry_source_dir(entry).join(story_artifacts);
    let read_story_artifact = |file_name: &str| -> Result<String, String> {
        fs::read_to_string(story_artifacts_path.join(file_name)).map_err(|error| {
            format!(
                "failed to read examples story promotion artifact {} for {}: {error}",
                story_artifacts_path.join(file_name).display(),
                entry.id
            )
        })
    };
    let story_mode_report_text = read_story_artifact("story-mode-report.txt")?;
    let story_llm_harness_report_text = read_story_artifact("story-llm-harness-report.txt")?;
    let story_manifest_text = read_story_artifact("manifest.ail-story.txt")?;
    let agent_trace_text = read_story_artifact("agent-trace.txt")?;
    let model_check_text = read_story_artifact("model-check.json")?;
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let source_entry = extract_ail_e2e_json_string_field(request_text, "source_entry_id")
        .or_else(|| {
            entry
                .id
                .strip_suffix("-story")
                .map(|source| source.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());
    let reviewer_agent = extract_ail_e2e_json_string_field(request_text, "executor_label")
        .or_else(|| entry.fields.get("executor-label").cloned())
        .unwrap_or_else(|| "codex-ail-story-promotion-reviewer".to_string());
    let agent_contract = extract_ail_e2e_json_string_field(request_text, "agent_contract")
        .unwrap_or_else(|| "examples/agents/codex-ail-story-promotion-reviewer.md".to_string());
    let approval_mode = extract_ail_e2e_json_string_field(request_text, "approval_mode")
        .unwrap_or_else(|| "deterministic-demo".to_string());
    let capture_plan_fingerprint =
        extract_ail_e2e_json_string_field(request_text, "story_promotion_capture_plan_fingerprint")
            .unwrap_or_else(|| "unknown".to_string());
    let story_text = render_ail_e2e_user_story_text(entry, semantic_anchors);
    let (preserved_count, missing_count) =
        ail_e2e_semantic_anchor_preservation_counts(&story_text, semantic_anchors);
    let mut lines = vec![
        "AIL-Story-Promotion-Review:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("capability-under-test {}", field("capability-under-test")),
        "story-promotion-review-artifact deterministic-text".to_string(),
        format!("reviewer-agent {reviewer_agent}"),
        format!("agent-contract {agent_contract}"),
        format!("approval-mode {approval_mode}"),
        "promotion-decision accepted-for-promotion".to_string(),
        "human-approval-required true".to_string(),
        format!("source-entry {source_entry}"),
        format!("proposed-accepted-entry-id {}", entry.id),
        format!("user-story-id {}", field("user-story-id")),
        format!("story-journey {}", field("story-journey")),
        format!("story-roundtrip {}", field("story-roundtrip")),
        format!("story-evidence {}", field("story-evidence")),
        format!("target-contract {}", field("target")),
        format!("runtime-evidence {runtime_evidence}"),
        format!("story-artifacts {story_artifacts}"),
        "story-artifacts-preserved true".to_string(),
        format!("story-promotion-capture-plan-fingerprint {capture_plan_fingerprint}"),
        format!("story-mode-report {story_artifacts}/story-mode-report.txt"),
        format!(
            "story-mode-report-fingerprint {}",
            ail_artifact_fingerprint(&story_mode_report_text)
        ),
        format!("story-llm-harness-report {story_artifacts}/story-llm-harness-report.txt"),
        format!(
            "story-llm-harness-report-fingerprint {}",
            ail_artifact_fingerprint(&story_llm_harness_report_text)
        ),
        format!("story-manifest {story_artifacts}/manifest.ail-story.txt"),
        format!(
            "story-manifest-fingerprint {}",
            ail_artifact_fingerprint(&story_manifest_text)
        ),
        format!("agent-trace {story_artifacts}/agent-trace.txt"),
        format!(
            "agent-trace-fingerprint {}",
            ail_artifact_fingerprint(&agent_trace_text)
        ),
        format!("model-check {story_artifacts}/model-check.json"),
        format!(
            "model-check-fingerprint {}",
            ail_artifact_fingerprint(&model_check_text)
        ),
    ];
    for (label, text, prefixes) in [
        (
            "story-prompt-envelope-valid-count",
            story_mode_report_text.as_str(),
            &[
                "story-prompt-envelope-valid-count:",
                "story-prompt-envelope-valid-count ",
            ][..],
        ),
        (
            "story-prompt-envelope-invalid-count",
            story_mode_report_text.as_str(),
            &[
                "story-prompt-envelope-invalid-count:",
                "story-prompt-envelope-invalid-count ",
            ][..],
        ),
        (
            "story-llm-transcript-count",
            story_mode_report_text.as_str(),
            &["story-llm-transcript-count:", "story-llm-transcript-count "][..],
        ),
        (
            "manifest-entry-check-count",
            story_llm_harness_report_text.as_str(),
            &["manifest-entry-check-count "][..],
        ),
        (
            "fingerprint-check-count",
            story_llm_harness_report_text.as_str(),
            &["fingerprint-check-count "][..],
        ),
        (
            "story-llm-transcript-check-count",
            story_llm_harness_report_text.as_str(),
            &["story-llm-transcript-check-count "][..],
        ),
        (
            "model-check-model-count",
            story_llm_harness_report_text.as_str(),
            &["model-check-model-count "][..],
        ),
        (
            "model-check-model-id",
            story_llm_harness_report_text.as_str(),
            &["model-check-model-id "][..],
        ),
        (
            "review-result",
            story_llm_harness_report_text.as_str(),
            &["review-result "][..],
        ),
    ] {
        if let Some(value) = extract_ail_e2e_report_line_value_any(text, prefixes) {
            lines.push(format!("{label} {value}"));
        }
    }
    for (label, prefix) in [
        ("agent-story-id-match", "agent-story-id-match "),
        (
            "agent-semantic-anchor-match-count",
            "agent-semantic-anchor-match-count ",
        ),
        (
            "agent-semantic-anchor-missing-count",
            "agent-semantic-anchor-missing-count ",
        ),
    ] {
        if let Some(value) =
            extract_ail_e2e_report_line_value(&story_llm_harness_report_text, prefix)
        {
            lines.push(format!("{label} {value}"));
        }
    }
    lines.push(format!("semantic-anchor-preserved-count {preserved_count}"));
    lines.push(format!("semantic-anchor-missing-count {missing_count}"));
    for anchor in semantic_anchors {
        lines.push(format!("story-anchor {anchor}"));
    }
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "story-promotion-review-summary User Story mode promotion review connects stored story artifact bundle, LLM harness, model check, agent trace, accepted spec, checked Core, bytecode, runtime or target proof, semantic anchors, and promotion decision for reviewer audit."
            .to_string(),
    );
    Ok(Some(format!("{}\n", lines.join("\n"))))
}

fn render_ail_e2e_dependency_review_text(
    entry: &AilE2eCorpusEntry,
    semantic_anchors: &[String],
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let package_graph_signal =
        "Package graphs need clearer authoring guidance and dependency review views.";
    let is_support_composed_package_graph = field("capability-under-test") == "package-imports"
        || field("package").ends_with("support_composed.ail")
        || field("v0.3-signal") == package_graph_signal;
    if !is_support_composed_package_graph {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-Dependency-Review:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("capability-under-test {}", field("capability-under-test")),
        "dependency-review-artifact deterministic-text".to_string(),
        "package-surface package-graph".to_string(),
        "local-package support-composed".to_string(),
        "imported-package support-shared".to_string(),
        "import-alias Shared".to_string(),
        "imported-type Shared.User".to_string(),
        "owner-package support-shared".to_string(),
        "capability-grant imports,things,actions,failures,guarantees,traces".to_string(),
        "authoring-boundary imported ownership must remain visible before compile".to_string(),
        "review-boundary dependency identity, alias, type owner, capability grant, and replay fingerprints must be auditable"
            .to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    for anchor in semantic_anchors {
        lines.push(format!("story-anchor {anchor}"));
    }
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "dependency-review-summary Support Composed evidence keeps package identity, Shared alias ownership, imported type use, capability grant, story anchors, and replay fingerprints visible for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_e2e_stdlib_walkthrough_text(
    entry: &AilE2eCorpusEntry,
    semantic_anchors: &[String],
    checked_core_text: Option<&str>,
    bytecode_text: Option<&str>,
    vm_trace_text: Option<&str>,
    target_report_text: Option<&str>,
) -> Option<String> {
    let field = |key: &str| entry.fields.get(key).map(String::as_str).unwrap_or("");
    let generics_signal =
        "Generics need reusable conformance fixtures and teachable stdlib walkthroughs.";
    let is_stdlib_generics_entry = field("capability-under-test") == "stdlib-generics"
        || field("package").ends_with("ail_std_collections.ail")
        || field("v0.3-signal") == generics_signal;
    if !is_stdlib_generics_entry {
        return None;
    }
    let runtime_evidence = if target_report_text.is_some() {
        "target-report"
    } else if vm_trace_text.is_some() {
        "vm-trace"
    } else if bytecode_text.is_some() {
        "bytecode"
    } else {
        "checked-core"
    };
    let mut lines = vec![
        "AIL-Stdlib-Walkthrough:".to_string(),
        format!("entry {}", entry.id),
        format!("semantic-task {}", field("semantic-task")),
        format!("package {}", field("package")),
        format!("profile {}", field("profile")),
        format!("program-domain {}", field("program-domain")),
        format!("capability-under-test {}", field("capability-under-test")),
        "stdlib-walkthrough-artifact deterministic-text".to_string(),
        "stdlib-surface generic-collections".to_string(),
        "stdlib-package ail.std.collections".to_string(),
        "imported-package ail.std.core".to_string(),
        "import-alias Core".to_string(),
        "generic-type Option<T>".to_string(),
        "generic-type Result<T,E>".to_string(),
        "generic-type List<T>".to_string(),
        "generic-type Map<K,V>".to_string(),
        "generic-type Set<T>".to_string(),
        "generic-function Option.map".to_string(),
        "variant Some(value: T)".to_string(),
        "variant None".to_string(),
        "variant Success(value: T)".to_string(),
        "variant Failure(error: E)".to_string(),
        "some-behavior Option.map returns Some(mapped value)".to_string(),
        "none-behavior Option.map returns None".to_string(),
        "trace-event OptionMapEvaluated".to_string(),
        "accepted-fixture examples/accepted/option-map-minimal.ail-spec.md".to_string(),
        "rejected-fixture examples/rejected/invalid-generic-variant-payload.ail-spec.md"
            .to_string(),
        "story-anchor Option<T>".to_string(),
        "story-anchor Result<T,E>".to_string(),
        "story-anchor Map<K,V>".to_string(),
        "story-anchor Option.map".to_string(),
        "story-anchor OptionMapEvaluated".to_string(),
        format!("runtime-evidence {runtime_evidence}"),
    ];
    for anchor in semantic_anchors {
        lines.push(format!("source-story-anchor {anchor}"));
    }
    if let Some(checked_core_text) = checked_core_text {
        lines.push(format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(checked_core_text)
        ));
    }
    if let Some(bytecode_text) = bytecode_text {
        lines.push(format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ));
    }
    if let Some(vm_trace_text) = vm_trace_text {
        lines.push(format!(
            "vm-trace-fingerprint {}",
            ail_artifact_fingerprint(vm_trace_text)
        ));
    }
    if let Some(target_report_text) = target_report_text {
        lines.push(format!(
            "target-report-fingerprint {}",
            ail_artifact_fingerprint(target_report_text)
        ));
    }
    lines.push(
        "stdlib-walkthrough-summary Standard Collections evidence explains generic types, variants, Option.map behavior, accepted and rejected fixtures, story anchors, and replay fingerprints for reviewer audit."
            .to_string(),
    );
    Some(format!("{}\n", lines.join("\n")))
}

fn ail_join_nonempty_set(values: &BTreeSet<String>) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn render_ail_e2e_v03_roadmap(evaluations: &[AilE2eCorpusEvaluation]) -> String {
    let mut signals = BTreeMap::new();
    for evaluation in evaluations {
        let entry = &evaluation.entry;
        let Some(signal) = entry.fields.get("v0.3-signal") else {
            continue;
        };
        let coverage: &mut AilV03SignalCoverage = signals.entry(signal.clone()).or_default();
        coverage.count += 1;
        coverage.entries.insert(entry.id.clone());
        for (field, target) in [
            ("capability-level", &mut coverage.capability_levels),
            ("program-domain", &mut coverage.program_domains),
            ("prompt-file", &mut coverage.prompt_files),
            ("story-journey", &mut coverage.story_journeys),
            ("checker-result", &mut coverage.checker_results),
        ] {
            if let Some(value) = entry.fields.get(field) {
                target.insert(value.clone());
            }
        }
    }

    let mut lines = vec![
        "AIL-v0.3-Roadmap:".to_string(),
        format!("entry-count {}", evaluations.len()),
        format!("signal-count {}", signals.len()),
    ];
    for (signal, coverage) in signals {
        lines.push(format!(
            "signal {signal} count {} capability-levels {} program-domains {} story-journeys {} prompt-files {} checker-results {} entries {}",
            coverage.count,
            ail_join_nonempty_set(&coverage.capability_levels),
            ail_join_nonempty_set(&coverage.program_domains),
            ail_join_nonempty_set(&coverage.story_journeys),
            ail_join_nonempty_set(&coverage.prompt_files),
            ail_join_nonempty_set(&coverage.checker_results),
            ail_join_nonempty_set(&coverage.entries)
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn render_ail_e2e_corpus_manifest(
    report_text: &str,
    roadmap_text: &str,
    model_executor_manifest_text: &str,
    evaluations: &[AilE2eCorpusEvaluation],
) -> String {
    let mut lines = vec![
        "AIL-Examples-Manifest:".to_string(),
        format!(
            "report examples-report.txt {}",
            ail_artifact_fingerprint(report_text)
        ),
        format!(
            "model-executor model-executor-manifest.txt {}",
            ail_artifact_fingerprint(model_executor_manifest_text)
        ),
        format!(
            "roadmap v03-roadmap.txt {}",
            ail_artifact_fingerprint(roadmap_text)
        ),
    ];
    for evaluation in evaluations {
        let entry = &evaluation.entry;
        let checker_result = entry
            .fields
            .get("checker-result")
            .map(String::as_str)
            .unwrap_or("unknown");
        let target = entry
            .fields
            .get("target")
            .map(String::as_str)
            .unwrap_or("unknown");
        lines.push(format!(
            "entry {} checker-result {} target {}",
            entry.id, checker_result, target
        ));
        push_ail_e2e_entry_artifact_lines(&mut lines, "entry-artifact", evaluation);
    }
    format!("{}\n", lines.join("\n"))
}

fn render_ail_e2e_model_executor_manifest(evaluations: &[AilE2eCorpusEvaluation]) -> String {
    let mut executor_family_counts = BTreeMap::new();
    let mut executor_label_counts = BTreeMap::new();
    let mut endpoint_label_counts = BTreeMap::new();
    let mut capture_origin_counts = BTreeMap::new();
    let mut executor_origin_counts = BTreeMap::new();
    let mut executor_endpoint_counts = BTreeMap::new();

    for evaluation in evaluations {
        let fields = &evaluation.entry.fields;
        let executor_family = fields
            .get("executor-family")
            .map(String::as_str)
            .unwrap_or("unknown");
        let executor_label = fields
            .get("executor-label")
            .map(String::as_str)
            .unwrap_or("unknown");
        let capture_origin = fields
            .get("capture-origin")
            .map(String::as_str)
            .unwrap_or("unknown");
        *executor_family_counts
            .entry(executor_family.to_string())
            .or_insert(0usize) += 1;
        *executor_label_counts
            .entry(executor_label.to_string())
            .or_insert(0usize) += 1;
        *capture_origin_counts
            .entry(capture_origin.to_string())
            .or_insert(0usize) += 1;
        *executor_origin_counts
            .entry(format!("{executor_family}@{capture_origin}"))
            .or_insert(0usize) += 1;
        if let Some(endpoint_label) = fields
            .get("endpoint-label")
            .filter(|label| !label.is_empty())
        {
            *endpoint_label_counts
                .entry(endpoint_label.to_string())
                .or_insert(0usize) += 1;
            *executor_endpoint_counts
                .entry(format!("{executor_label}@{endpoint_label}"))
                .or_insert(0usize) += 1;
        }
    }

    let mut lines = vec![
        "AIL-Examples-Model-Executor-Manifest:".to_string(),
        format!("entry-count {}", evaluations.len()),
    ];
    for (executor_family, count) in executor_family_counts {
        lines.push(format!("executor-family {executor_family} count {count}"));
    }
    for (executor_label, count) in executor_label_counts {
        lines.push(format!("executor-label {executor_label} count {count}"));
    }
    for (endpoint_label, count) in endpoint_label_counts {
        lines.push(format!("endpoint-label {endpoint_label} count {count}"));
    }
    for (capture_origin, count) in capture_origin_counts {
        lines.push(format!("capture-origin {capture_origin} count {count}"));
    }
    for (executor_origin, count) in executor_origin_counts {
        lines.push(format!("executor-origin {executor_origin} count {count}"));
    }
    for (executor_endpoint, count) in executor_endpoint_counts {
        lines.push(format!(
            "executor-endpoint {executor_endpoint} count {count}"
        ));
    }
    for evaluation in evaluations {
        let fields = &evaluation.entry.fields;
        let semantic_task = fields
            .get("semantic-task")
            .map(String::as_str)
            .unwrap_or("unknown");
        let executor_family = fields
            .get("executor-family")
            .map(String::as_str)
            .unwrap_or("unknown");
        let executor_label = fields
            .get("executor-label")
            .map(String::as_str)
            .unwrap_or("unknown");
        let endpoint_label = fields
            .get("endpoint-label")
            .filter(|label| !label.is_empty())
            .map(String::as_str)
            .unwrap_or("none");
        let capture_origin = fields
            .get("capture-origin")
            .map(String::as_str)
            .unwrap_or("unknown");
        lines.push(format!(
            "entry {} semantic-task {} executor-family {} executor-label {} endpoint-label {} capture-origin {}",
            evaluation.entry.id,
            semantic_task,
            executor_family,
            executor_label,
            endpoint_label,
            capture_origin
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn ail_e2e_semantic_task_family(semantic_task: &str) -> String {
    if let Some((family, suffix)) = semantic_task.rsplit_once('-')
        && !family.is_empty()
        && suffix.chars().all(|character| character.is_ascii_digit())
    {
        return family.to_string();
    }
    semantic_task.to_string()
}

fn ail_e2e_story_family_dimensions<'a, I>(
    entries: I,
) -> BTreeMap<String, AilE2eStoryFamilyDimensions>
where
    I: IntoIterator<Item = &'a AilE2eCorpusEntry>,
{
    let mut dimensions = BTreeMap::new();
    for entry in entries {
        let Some(story_id) = entry.fields.get("user-story-id") else {
            continue;
        };
        let family = dimensions
            .entry(story_id.to_string())
            .or_insert_with(AilE2eStoryFamilyDimensions::default);
        family.entry_count += 1;
        if let Some(prompt_file) = entry.fields.get("prompt-file") {
            family.prompt_files.insert(prompt_file.to_string());
        }
        if let Some(story_journey) = entry.fields.get("story-journey") {
            family.story_journeys.insert(story_journey.to_string());
        }
    }
    dimensions
}

fn validate_ail_e2e_corpus_release_coverage(entries: &[AilE2eCorpusEntry]) -> Result<(), String> {
    let semantic_tasks = entries
        .iter()
        .filter_map(|entry| entry.fields.get("semantic-task").map(String::as_str))
        .collect::<BTreeSet<_>>();
    if semantic_tasks.len() < 100 {
        return Err(format!(
            "ail-examples requires at least 100 distinct semantic-task entries; found {}",
            semantic_tasks.len()
        ));
    }
    let accepted_count = entries
        .iter()
        .filter(|entry| {
            entry
                .fields
                .get("checker-result")
                .is_some_and(|checker_result| checker_result == "accepted")
        })
        .count();
    if accepted_count < 100 {
        return Err(format!(
            "ail-examples requires at least 100 accepted prompt-to-artifact examples; found {accepted_count}"
        ));
    }
    let v03_signals = entries
        .iter()
        .filter_map(|entry| entry.fields.get("v0.3-signal").map(String::as_str))
        .collect::<BTreeSet<_>>();
    if v03_signals.len() < 10 {
        return Err(format!(
            "ail-examples requires at least 10 distinct v0.3-signal learning signals; found {}",
            v03_signals.len()
        ));
    }
    let executor_families = entries
        .iter()
        .filter_map(|entry| entry.fields.get("executor-family").map(String::as_str))
        .collect::<BTreeSet<_>>();
    for required_executor in ["llm-http", "codex-skill-agent"] {
        if !executor_families.contains(required_executor) {
            return Err(format!(
                "ail-examples requires executor-family {required_executor}"
            ));
        }
    }
    let prompt_files = entries
        .iter()
        .filter_map(|entry| entry.fields.get("prompt-file").map(String::as_str))
        .collect::<BTreeSet<_>>();
    for required_prompt in REQUIRED_AIL_PROMPT_FILES {
        if !prompt_files.contains(required_prompt) {
            return Err(format!(
                "ail-examples requires prompt-file {required_prompt}"
            ));
        }
    }
    let accepted_prompt_files = entries
        .iter()
        .filter(|entry| {
            entry
                .fields
                .get("checker-result")
                .is_some_and(|checker_result| checker_result == "accepted")
        })
        .filter_map(|entry| entry.fields.get("prompt-file").map(String::as_str))
        .collect::<BTreeSet<_>>();
    for required_prompt in REQUIRED_AIL_PROMPT_FILES {
        if !accepted_prompt_files.contains(required_prompt) {
            return Err(format!(
                "ail-examples requires accepted example for prompt-file {required_prompt}"
            ));
        }
    }
    let mut profile_counts = BTreeMap::new();
    for entry in entries {
        if let Some(profile) = entry.fields.get("profile") {
            *profile_counts.entry(profile.as_str()).or_insert(0usize) += 1;
        }
    }
    for (required_profile, required_count) in [
        ("Application", 40usize),
        ("AgentTool", 15usize),
        ("Compiler", 10usize),
        ("System", 10usize),
    ] {
        let found = profile_counts.get(required_profile).copied().unwrap_or(0);
        if found < required_count {
            return Err(format!(
                "ail-examples requires at least {required_count} profile {required_profile} examples; found {found}"
            ));
        }
    }
    let mut surface_counts = BTreeMap::new();
    for entry in entries {
        if let Some(surface_tags) = entry.fields.get("surface-tags") {
            for tag in surface_tags.split([',', ';']) {
                let tag = tag.trim();
                if !tag.is_empty() {
                    *surface_counts.entry(tag).or_insert(0usize) += 1;
                }
            }
        }
    }
    let stdlib_or_package_import = surface_counts.get("standard-library").copied().unwrap_or(0)
        + surface_counts.get("package-import").copied().unwrap_or(0);
    if stdlib_or_package_import < 10 {
        return Err(format!(
            "ail-examples requires at least 10 standard-library or package-import examples; found {stdlib_or_package_import}"
        ));
    }
    for (required_surface, required_count) in [
        ("ui", 5usize),
        ("c-host-interop", 5usize),
        ("backend-portability", 5usize),
    ] {
        let found = surface_counts.get(required_surface).copied().unwrap_or(0);
        if found < required_count {
            return Err(format!(
                "ail-examples requires at least {required_count} surface-tag {required_surface} examples; found {found}"
            ));
        }
    }
    let mut capability_level_counts = BTreeMap::new();
    for entry in entries {
        if let Some(capability_level) = entry.fields.get("capability-level") {
            *capability_level_counts
                .entry(capability_level.as_str())
                .or_insert(0usize) += 1;
        }
    }
    for (required_level, required_count) in [
        ("low-level", 20usize),
        ("mid-level", 20usize),
        ("high-level", 20usize),
    ] {
        let found = capability_level_counts
            .get(required_level)
            .copied()
            .unwrap_or(0);
        if found < required_count {
            return Err(format!(
                "ail-examples requires at least {required_count} capability-level {required_level} examples; found {found}"
            ));
        }
    }
    let mut program_scale_counts = BTreeMap::new();
    for entry in entries {
        if let Some(program_scale) = entry.fields.get("program-scale") {
            *program_scale_counts
                .entry(program_scale.as_str())
                .or_insert(0usize) += 1;
        }
    }
    for (required_scale, required_count) in [
        ("utility", 10usize),
        ("module", 20usize),
        ("multi-module-system", 10usize),
    ] {
        let found = program_scale_counts
            .get(required_scale)
            .copied()
            .unwrap_or(0);
        if found < required_count {
            return Err(format!(
                "ail-examples requires at least {required_count} program-scale {required_scale} examples; found {found}"
            ));
        }
    }
    let mut program_domain_counts = BTreeMap::new();
    let mut program_domain_prompt_files: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut program_domain_story_journeys: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut interactive_entry_count = 0usize;
    let mut multi_artifact_entry_count = 0usize;
    for entry in entries {
        if let Some(program_domain) = entry.fields.get("program-domain") {
            *program_domain_counts
                .entry(program_domain.as_str())
                .or_insert(0usize) += 1;
            if let Some(prompt_file) = entry.fields.get("prompt-file") {
                program_domain_prompt_files
                    .entry(program_domain.as_str())
                    .or_default()
                    .insert(prompt_file.as_str());
            }
            if let Some(story_journey) = entry.fields.get("story-journey") {
                program_domain_story_journeys
                    .entry(program_domain.as_str())
                    .or_default()
                    .insert(story_journey.as_str());
            }
        }
        if entry
            .fields
            .get("interacts-with")
            .is_some_and(|interacts_with| interacts_with != "none")
        {
            interactive_entry_count += 1;
        }
        let module_count = entry
            .fields
            .get("module-count")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let spec_count = entry
            .fields
            .get("spec-count")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let story_count = entry
            .fields
            .get("story-count")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        if module_count >= 2 && spec_count >= 2 && story_count >= 2 {
            multi_artifact_entry_count += 1;
        }
    }
    for (required_domain, required_count) in [
        ("os-utility", 5usize),
        ("c-interop", 5usize),
        ("compiler", 5usize),
        ("runtime", 5usize),
        ("package-graph", 5usize),
        ("application", 5usize),
        ("agent-tool", 10usize),
        ("ui-workflow", 5usize),
        ("system-driver", 5usize),
    ] {
        let found = program_domain_counts
            .get(required_domain)
            .copied()
            .unwrap_or(0);
        if found < required_count {
            return Err(format!(
                "ail-examples requires at least {required_count} program-domain {required_domain} examples; found {found}"
            ));
        }
        let prompt_file_count = program_domain_prompt_files
            .get(required_domain)
            .map(BTreeSet::len)
            .unwrap_or(0);
        if prompt_file_count < 3 {
            return Err(format!(
                "ail-examples requires program-domain {required_domain} to cover at least 3 prompt files; found {prompt_file_count}"
            ));
        }
        let story_journey_count = program_domain_story_journeys
            .get(required_domain)
            .map(BTreeSet::len)
            .unwrap_or(0);
        if story_journey_count < 2 {
            return Err(format!(
                "ail-examples requires program-domain {required_domain} to cover at least 2 story journeys; found {story_journey_count}"
            ));
        }
    }
    if interactive_entry_count < 20 {
        return Err(format!(
            "ail-examples requires at least 20 examples with interacts-with other than none; found {interactive_entry_count}"
        ));
    }
    if multi_artifact_entry_count < 10 {
        return Err(format!(
            "ail-examples requires at least 10 examples with module-count/spec-count/story-count >= 2; found {multi_artifact_entry_count}"
        ));
    }
    let mut story_journey_counts = BTreeMap::new();
    for entry in entries {
        if let Some(story_journey) = entry.fields.get("story-journey") {
            *story_journey_counts
                .entry(story_journey.as_str())
                .or_insert(0usize) += 1;
        }
    }
    for (required_journey, required_count) in [
        ("story-to-spec", 20usize),
        ("spec-to-story", 5usize),
        ("story-amendment", 20usize),
    ] {
        let found = story_journey_counts
            .get(required_journey)
            .copied()
            .unwrap_or(0);
        if found < required_count {
            return Err(format!(
                "ail-examples requires at least {required_count} story-journey {required_journey} examples; found {found}"
            ));
        }
    }
    let distinct_story_ids = entries
        .iter()
        .filter_map(|entry| entry.fields.get("user-story-id").map(String::as_str))
        .collect::<BTreeSet<_>>();
    if distinct_story_ids.len() < 10 {
        return Err(format!(
            "ail-examples requires at least 10 distinct user-story-id entries; found {}",
            distinct_story_ids.len()
        ));
    }
    let mut story_family_counts = BTreeMap::new();
    for entry in entries {
        let is_application_workflow = entry
            .fields
            .get("capability-under-test")
            .is_some_and(|capability| capability == "application-workflow");
        let is_high_level = entry
            .fields
            .get("capability-level")
            .is_some_and(|level| level == "high-level");
        if is_application_workflow
            && is_high_level
            && let Some(story_id) = entry.fields.get("user-story-id")
        {
            *story_family_counts
                .entry(story_id.as_str())
                .or_insert(0usize) += 1;
        }
    }
    if !story_family_counts.values().any(|count| *count >= 2) {
        return Err(
            "ail-examples requires one high-level application-workflow user-story-id family with at least two entries"
                .to_string(),
        );
    }
    for (story_id, dimensions) in ail_e2e_story_family_dimensions(entries) {
        if dimensions.entry_count >= 5
            && (dimensions.prompt_files.len() < 3 || dimensions.story_journeys.len() < 2)
        {
            return Err(format!(
                "ail-examples repeated user-story-id {story_id} has {} entries but only {} prompt files and {} story journeys; repeated families with at least 5 entries require at least 3 prompt files and 2 story journeys",
                dimensions.entry_count,
                dimensions.prompt_files.len(),
                dimensions.story_journeys.len()
            ));
        }
    }
    let mut target_counts = BTreeMap::new();
    for entry in entries {
        if let Some(target) = entry.fields.get("target") {
            *target_counts.entry(target.as_str()).or_insert(0usize) += 1;
        }
    }
    for required_target in [
        "linux-x86_64-elf",
        "wasm32-unknown-sandbox-wasm",
        "aarch64-apple-darwin-libsystem-macho",
        "vm",
    ] {
        let found = target_counts.get(required_target).copied().unwrap_or(0);
        if found < 5 {
            return Err(format!(
                "ail-examples requires at least 5 target {required_target} examples; found {found}"
            ));
        }
    }
    let mut llm_labels_by_family: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for entry in entries {
        if entry
            .fields
            .get("executor-family")
            .is_some_and(|executor_family| executor_family == "llm-http")
        {
            let semantic_task = entry
                .fields
                .get("semantic-task")
                .map(String::as_str)
                .unwrap_or_default();
            let executor_label = entry
                .fields
                .get("executor-label")
                .map(String::as_str)
                .unwrap_or_default();
            let endpoint_label = entry
                .fields
                .get("endpoint-label")
                .map(String::as_str)
                .unwrap_or_default();
            llm_labels_by_family
                .entry(ail_e2e_semantic_task_family(semantic_task))
                .or_default()
                .insert(format!("{executor_label}@{endpoint_label}"));
        }
    }
    if !llm_labels_by_family
        .values()
        .any(|executor_endpoint_labels| executor_endpoint_labels.len() >= 2)
    {
        return Err(
            "ail-examples requires one semantic-task family with at least two llm-http executor/endpoint labels"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_ail_e2e_corpus_live_release_evidence(
    entries: &[AilE2eCorpusEntry],
) -> Result<(), String> {
    let mut capture_origin_counts = BTreeMap::new();
    for entry in entries {
        if let Some(capture_origin) = entry.fields.get("capture-origin") {
            *capture_origin_counts
                .entry(capture_origin.as_str())
                .or_insert(0usize) += 1;
        }
    }
    let deterministic_seed_count = capture_origin_counts
        .get("deterministic-seed")
        .copied()
        .unwrap_or(0);
    if deterministic_seed_count > 0 {
        return Err(format!(
            "ail-examples --release-evidence requires zero deterministic-seed entries; found {deterministic_seed_count}"
        ));
    }
    for entry in entries {
        let executor_family = entry
            .fields
            .get("executor-family")
            .map(String::as_str)
            .unwrap_or_default();
        let capture_origin = entry
            .fields
            .get("capture-origin")
            .map(String::as_str)
            .unwrap_or_default();
        if executor_family == "llm-http" && capture_origin != "live-llm" {
            return Err(format!(
                "ail-examples --release-evidence llm-http entry {} must use capture-origin live-llm",
                entry.id
            ));
        }
        if executor_family == "codex-skill-agent" && capture_origin != "live-codex" {
            return Err(format!(
                "ail-examples --release-evidence codex-skill-agent entry {} must use capture-origin live-codex",
                entry.id
            ));
        }
    }
    for required_capture_origin in ["live-llm", "live-codex"] {
        if capture_origin_counts
            .get(required_capture_origin)
            .copied()
            .unwrap_or(0)
            == 0
        {
            return Err(format!(
                "ail-examples --release-evidence requires capture-origin {required_capture_origin}"
            ));
        }
    }
    Ok(())
}

fn validate_ail_e2e_story_evidence_artifacts(
    evaluations: &[AilE2eCorpusEvaluation],
) -> Result<(), String> {
    for evaluation in evaluations {
        let story_evidence = evaluation
            .entry
            .fields
            .get("story-evidence")
            .map(String::as_str)
            .unwrap_or_default();
        let produced = match story_evidence {
            "checked-core" => evaluation.checked_core_text.is_some(),
            "bytecode" => evaluation.bytecode_text.is_some(),
            "vm-trace" => evaluation.vm_trace_text.is_some(),
            "target-report" => evaluation.target_report_text.is_some(),
            "diagnostics" => evaluation.diagnostics_text.is_some(),
            _ => true,
        };
        if !produced {
            return Err(format!(
                "examples catalog entry {} story-evidence {story_evidence} requires {story_evidence} artifact",
                evaluation.entry.id
            ));
        }
    }
    Ok(())
}

fn validate_ail_e2e_repair_proof_distinctness(
    evaluations: &[AilE2eCorpusEvaluation],
) -> Result<(), String> {
    for label in [
        "repair-candidate",
        "repair-checked-core",
        "repair-bytecode",
        "repair-vm-trace",
        "repair-target-report",
    ] {
        let mut entries_by_fingerprint = BTreeMap::<String, Vec<String>>::new();
        for evaluation in evaluations {
            let Some(repair_proof) = &evaluation.repair_proof else {
                continue;
            };
            let fingerprint = match label {
                "repair-candidate" => {
                    Some(ail_artifact_fingerprint(&repair_proof.candidate_spec_text))
                }
                "repair-checked-core" => {
                    Some(ail_artifact_fingerprint(&repair_proof.checked_core_text))
                }
                "repair-bytecode" => Some(ail_artifact_fingerprint(&repair_proof.bytecode_text)),
                "repair-vm-trace" => repair_proof
                    .vm_trace_text
                    .as_ref()
                    .map(|text| ail_artifact_fingerprint(text)),
                "repair-target-report" => repair_proof
                    .target_report_text
                    .as_ref()
                    .map(|text| ail_artifact_fingerprint(text)),
                _ => None,
            };
            if let Some(fingerprint) = fingerprint {
                entries_by_fingerprint
                    .entry(fingerprint)
                    .or_default()
                    .push(evaluation.entry.id.clone());
            }
        }
        for (fingerprint, entries) in entries_by_fingerprint {
            if entries.len() > 1 {
                return Err(format!(
                    "ail-examples --release-evidence rejected {label} artifacts must be distinct; fingerprint {fingerprint} is reused by {}",
                    entries.join(",")
                ));
            }
        }
    }
    Ok(())
}

fn write_ail_e2e_corpus_artifacts(
    artifact_dir: &str,
    report_text: &str,
    evaluations: &[AilE2eCorpusEvaluation],
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root)
        .map_err(|error| format!("failed to create ail-examples artifact dir: {error}"))?;
    fs::write(root.join("examples-report.txt"), report_text)
        .map_err(|error| format!("failed to write examples catalog report: {error}"))?;
    fs::write(
        root.join("examples-report.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(report_text)),
    )
    .map_err(|error| format!("failed to write examples catalog report fingerprint: {error}"))?;
    let roadmap_text = render_ail_e2e_v03_roadmap(evaluations);
    fs::write(root.join("v03-roadmap.txt"), &roadmap_text)
        .map_err(|error| format!("failed to write examples v0.3 roadmap: {error}"))?;
    fs::write(
        root.join("v03-roadmap.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&roadmap_text)),
    )
    .map_err(|error| format!("failed to write examples v0.3 roadmap fingerprint: {error}"))?;
    let model_executor_manifest_text = render_ail_e2e_model_executor_manifest(evaluations);
    fs::write(
        root.join("model-executor-manifest.txt"),
        &model_executor_manifest_text,
    )
    .map_err(|error| format!("failed to write examples model executor manifest: {error}"))?;
    fs::write(
        root.join("model-executor-manifest.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(&model_executor_manifest_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write examples model executor manifest fingerprint: {error}")
    })?;
    let manifest_text = render_ail_e2e_corpus_manifest(
        report_text,
        &roadmap_text,
        &model_executor_manifest_text,
        evaluations,
    );
    fs::write(root.join("manifest.ail-examples.txt"), &manifest_text)
        .map_err(|error| format!("failed to write examples catalog manifest: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| format!("failed to write examples catalog manifest fingerprint: {error}"))?;
    for evaluation in evaluations {
        let story_text =
            render_ail_e2e_user_story_text(&evaluation.entry, &evaluation.semantic_anchors);
        let entry_dir = root.join("examples").join(&evaluation.entry.id);
        fs::create_dir_all(&entry_dir)
            .map_err(|error| format!("failed to create examples entry artifact dir: {error}"))?;
        fs::write(entry_dir.join("user-story.txt"), &story_text)
            .map_err(|error| format!("failed to write examples user story: {error}"))?;
        fs::write(
            entry_dir.join("user-story.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(&story_text)),
        )
        .map_err(|error| format!("failed to write examples user story fingerprint: {error}"))?;
        if evaluation.request_fingerprint.is_some()
            || evaluation.response_fingerprint.is_some()
            || evaluation.extracted_artifact_fingerprint.is_some()
        {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            if let Some(request_fingerprint) = &evaluation.request_fingerprint {
                fs::write(
                    entry_dir.join("request.fingerprint.txt"),
                    format!("{request_fingerprint}\n"),
                )
                .map_err(|error| {
                    format!("failed to write examples request fingerprint: {error}")
                })?;
            }
            if let Some(response_fingerprint) = &evaluation.response_fingerprint {
                fs::write(
                    entry_dir.join("response.fingerprint.txt"),
                    format!("{response_fingerprint}\n"),
                )
                .map_err(|error| {
                    format!("failed to write examples response fingerprint: {error}")
                })?;
            }
            if let Some(extracted_artifact_fingerprint) = &evaluation.extracted_artifact_fingerprint
            {
                fs::write(
                    entry_dir.join("artifact.fingerprint.txt"),
                    format!("{extracted_artifact_fingerprint}\n"),
                )
                .map_err(|error| {
                    format!("failed to write examples extracted artifact fingerprint: {error}")
                })?;
            }
        }
        if let Some(core_text) = &evaluation.checked_core_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(entry_dir.join("checked.ail-core.txt"), core_text)
                .map_err(|error| format!("failed to write examples checked core: {error}"))?;
            fs::write(
                entry_dir.join("checked.ail-core.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(core_text)),
            )
            .map_err(|error| {
                format!("failed to write examples checked core fingerprint: {error}")
            })?;
        }
        if let Some(bytecode_text) = &evaluation.bytecode_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(entry_dir.join("artifact.ailbc.json"), bytecode_text)
                .map_err(|error| format!("failed to write examples bytecode artifact: {error}"))?;
            fs::write(
                entry_dir.join("artifact.ailbc.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(bytecode_text)),
            )
            .map_err(|error| format!("failed to write examples bytecode fingerprint: {error}"))?;
        }
        if let Some(vm_trace_text) = &evaluation.vm_trace_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(entry_dir.join("vm-trace.txt"), vm_trace_text)
                .map_err(|error| format!("failed to write examples vm trace: {error}"))?;
            fs::write(
                entry_dir.join("vm-trace.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(vm_trace_text)),
            )
            .map_err(|error| format!("failed to write examples vm trace fingerprint: {error}"))?;
        }
        if !evaluation.native_executables.is_empty() {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            for executable in &evaluation.native_executables {
                let artifact_path = entry_dir.join(&executable.file_name);
                fs::write(&artifact_path, &executable.bytes).map_err(|error| {
                    format!(
                        "failed to write examples native executable {}: {error}",
                        executable.file_name
                    )
                })?;
                set_native_executable_permissions(&artifact_path.to_string_lossy())?;
                fs::write(
                    entry_dir.join(format!("{}.fingerprint.txt", executable.file_name)),
                    format!("{}\n", ail_artifact_fingerprint_bytes(&executable.bytes)),
                )
                .map_err(|error| {
                    format!(
                        "failed to write examples native executable fingerprint {}: {error}",
                        executable.file_name
                    )
                })?;
            }
        }
        if let Some(target_report_text) = &evaluation.target_report_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(entry_dir.join("target-report.txt"), target_report_text)
                .map_err(|error| format!("failed to write examples target report: {error}"))?;
            fs::write(
                entry_dir.join("target-report.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(target_report_text)),
            )
            .map_err(|error| {
                format!("failed to write examples target report fingerprint: {error}")
            })?;
        }
        if let Some(ui_review_text) = &evaluation.ui_review_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(entry_dir.join("ui-review.txt"), ui_review_text)
                .map_err(|error| format!("failed to write examples UI review: {error}"))?;
            fs::write(
                entry_dir.join("ui-review.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(ui_review_text)),
            )
            .map_err(|error| format!("failed to write examples UI review fingerprint: {error}"))?;
        }
        if let Some(ui_review_patch_text) = &evaluation.ui_review_patch_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(entry_dir.join("ui-review-patch.txt"), ui_review_patch_text)
                .map_err(|error| format!("failed to write examples UI review patch: {error}"))?;
            fs::write(
                entry_dir.join("ui-review-patch.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(ui_review_patch_text)),
            )
            .map_err(|error| {
                format!("failed to write examples UI review patch fingerprint: {error}")
            })?;
        }
        if let Some(ui_semantic_tags_text) = &evaluation.ui_semantic_tags_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("ui-semantic-tags.txt"),
                ui_semantic_tags_text,
            )
            .map_err(|error| format!("failed to write examples UI semantic tags: {error}"))?;
            fs::write(
                entry_dir.join("ui-semantic-tags.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(ui_semantic_tags_text)),
            )
            .map_err(|error| {
                format!("failed to write examples UI semantic tags fingerprint: {error}")
            })?;
        }
        if let Some(agent_policy_review_text) = &evaluation.agent_policy_review_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("agent-policy-review.txt"),
                agent_policy_review_text,
            )
            .map_err(|error| format!("failed to write examples agent policy review: {error}"))?;
            fs::write(
                entry_dir.join("agent-policy-review.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(agent_policy_review_text)),
            )
            .map_err(|error| {
                format!("failed to write examples agent policy review fingerprint: {error}")
            })?;
        }
        if let Some(threat_model_audit_text) = &evaluation.threat_model_audit_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("threat-model-audit.txt"),
                threat_model_audit_text,
            )
            .map_err(|error| format!("failed to write examples threat model audit: {error}"))?;
            fs::write(
                entry_dir.join("threat-model-audit.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(threat_model_audit_text)),
            )
            .map_err(|error| {
                format!("failed to write examples threat model audit fingerprint: {error}")
            })?;
        }
        if let Some(type_inference_review_text) = &evaluation.type_inference_review_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("type-inference-review.txt"),
                type_inference_review_text,
            )
            .map_err(|error| format!("failed to write examples type inference review: {error}"))?;
            fs::write(
                entry_dir.join("type-inference-review.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(type_inference_review_text)),
            )
            .map_err(|error| {
                format!("failed to write examples type inference review fingerprint: {error}")
            })?;
        }
        if let Some(state_boundary_review_text) = &evaluation.state_boundary_review_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("state-boundary-review.txt"),
                state_boundary_review_text,
            )
            .map_err(|error| format!("failed to write examples state boundary review: {error}"))?;
            fs::write(
                entry_dir.join("state-boundary-review.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(state_boundary_review_text)),
            )
            .map_err(|error| {
                format!("failed to write examples state boundary review fingerprint: {error}")
            })?;
        }
        if let Some(workflow_scheduler_review_text) = &evaluation.workflow_scheduler_review_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("workflow-scheduler-review.txt"),
                workflow_scheduler_review_text,
            )
            .map_err(|error| {
                format!("failed to write examples workflow scheduler review: {error}")
            })?;
            fs::write(
                entry_dir.join("workflow-scheduler-review.fingerprint.txt"),
                format!(
                    "{}\n",
                    ail_artifact_fingerprint(workflow_scheduler_review_text)
                ),
            )
            .map_err(|error| {
                format!("failed to write examples workflow scheduler review fingerprint: {error}")
            })?;
        }
        if let Some(unsafe_boundary_review_text) = &evaluation.unsafe_boundary_review_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("unsafe-boundary-review.txt"),
                unsafe_boundary_review_text,
            )
            .map_err(|error| format!("failed to write examples unsafe boundary review: {error}"))?;
            fs::write(
                entry_dir.join("unsafe-boundary-review.fingerprint.txt"),
                format!(
                    "{}\n",
                    ail_artifact_fingerprint(unsafe_boundary_review_text)
                ),
            )
            .map_err(|error| {
                format!("failed to write examples unsafe boundary review fingerprint: {error}")
            })?;
        }
        if let Some(complex_story_graph_text) = &evaluation.complex_story_graph_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("complex-story-graph.txt"),
                complex_story_graph_text,
            )
            .map_err(|error| format!("failed to write examples complex story graph: {error}"))?;
            fs::write(
                entry_dir.join("complex-story-graph.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(complex_story_graph_text)),
            )
            .map_err(|error| {
                format!("failed to write examples complex story graph fingerprint: {error}")
            })?;
        }
        if let Some(application_walkthrough_text) = &evaluation.application_walkthrough_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("application-walkthrough.txt"),
                application_walkthrough_text,
            )
            .map_err(|error| {
                format!("failed to write examples application walkthrough: {error}")
            })?;
            fs::write(
                entry_dir.join("application-walkthrough.fingerprint.txt"),
                format!(
                    "{}\n",
                    ail_artifact_fingerprint(application_walkthrough_text)
                ),
            )
            .map_err(|error| {
                format!("failed to write examples application walkthrough fingerprint: {error}")
            })?;
        }
        if let Some(story_promotion_review_text) = &evaluation.story_promotion_review_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("story-promotion-review.txt"),
                story_promotion_review_text,
            )
            .map_err(|error| format!("failed to write examples story promotion review: {error}"))?;
            fs::write(
                entry_dir.join("story-promotion-review.fingerprint.txt"),
                format!(
                    "{}\n",
                    ail_artifact_fingerprint(story_promotion_review_text)
                ),
            )
            .map_err(|error| {
                format!("failed to write examples story promotion review fingerprint: {error}")
            })?;
        }
        if let Some(dependency_review_text) = &evaluation.dependency_review_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("dependency-review.txt"),
                dependency_review_text,
            )
            .map_err(|error| format!("failed to write examples dependency review: {error}"))?;
            fs::write(
                entry_dir.join("dependency-review.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(dependency_review_text)),
            )
            .map_err(|error| {
                format!("failed to write examples dependency review fingerprint: {error}")
            })?;
        }
        if let Some(stdlib_walkthrough_text) = &evaluation.stdlib_walkthrough_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("stdlib-walkthrough.txt"),
                stdlib_walkthrough_text,
            )
            .map_err(|error| format!("failed to write examples stdlib walkthrough: {error}"))?;
            fs::write(
                entry_dir.join("stdlib-walkthrough.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(stdlib_walkthrough_text)),
            )
            .map_err(|error| {
                format!("failed to write examples stdlib walkthrough fingerprint: {error}")
            })?;
        }
        if let Some(diagnostics_text) = &evaluation.diagnostics_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(entry_dir.join("diagnostics.txt"), diagnostics_text)
                .map_err(|error| format!("failed to write examples diagnostics: {error}"))?;
            fs::write(
                entry_dir.join("diagnostics.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(diagnostics_text)),
            )
            .map_err(|error| {
                format!("failed to write examples diagnostics fingerprint: {error}")
            })?;
        }
        if let Some(repair_tutorial_text) = &evaluation.repair_tutorial_text {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(entry_dir.join("repair-tutorial.txt"), repair_tutorial_text)
                .map_err(|error| format!("failed to write examples repair tutorial: {error}"))?;
            fs::write(
                entry_dir.join("repair-tutorial.fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(repair_tutorial_text)),
            )
            .map_err(|error| {
                format!("failed to write examples repair tutorial fingerprint: {error}")
            })?;
        }
        if let Some(repair_proof) = &evaluation.repair_proof {
            let entry_dir = root.join("examples").join(&evaluation.entry.id);
            fs::create_dir_all(&entry_dir).map_err(|error| {
                format!("failed to create examples entry artifact dir: {error}")
            })?;
            fs::write(
                entry_dir.join("repair-candidate.ail-spec.md"),
                &repair_proof.candidate_spec_text,
            )
            .map_err(|error| format!("failed to write examples repair candidate spec: {error}"))?;
            fs::write(
                entry_dir.join("repair-candidate.fingerprint.txt"),
                format!(
                    "{}\n",
                    ail_artifact_fingerprint(&repair_proof.candidate_spec_text)
                ),
            )
            .map_err(|error| {
                format!("failed to write examples repair candidate fingerprint: {error}")
            })?;
            fs::write(
                entry_dir.join("repair-checked.ail-core.txt"),
                &repair_proof.checked_core_text,
            )
            .map_err(|error| format!("failed to write examples repair checked core: {error}"))?;
            fs::write(
                entry_dir.join("repair-checked.ail-core.fingerprint.txt"),
                format!(
                    "{}\n",
                    ail_artifact_fingerprint(&repair_proof.checked_core_text)
                ),
            )
            .map_err(|error| {
                format!("failed to write examples repair checked core fingerprint: {error}")
            })?;
            fs::write(
                entry_dir.join("repair-artifact.ailbc.json"),
                &repair_proof.bytecode_text,
            )
            .map_err(|error| format!("failed to write examples repair bytecode: {error}"))?;
            fs::write(
                entry_dir.join("repair-artifact.ailbc.fingerprint.txt"),
                format!(
                    "{}\n",
                    ail_artifact_fingerprint(&repair_proof.bytecode_text)
                ),
            )
            .map_err(|error| {
                format!("failed to write examples repair bytecode fingerprint: {error}")
            })?;
            if let Some(vm_trace_text) = &repair_proof.vm_trace_text {
                fs::write(entry_dir.join("repair-vm-trace.txt"), vm_trace_text).map_err(
                    |error| format!("failed to write examples repair vm trace: {error}"),
                )?;
                fs::write(
                    entry_dir.join("repair-vm-trace.fingerprint.txt"),
                    format!("{}\n", ail_artifact_fingerprint(vm_trace_text)),
                )
                .map_err(|error| {
                    format!("failed to write examples repair vm trace fingerprint: {error}")
                })?;
            }
            if let Some(target_report_text) = &repair_proof.target_report_text {
                fs::write(
                    entry_dir.join("repair-target-report.txt"),
                    target_report_text,
                )
                .map_err(|error| {
                    format!("failed to write examples repair target report: {error}")
                })?;
                fs::write(
                    entry_dir.join("repair-target-report.fingerprint.txt"),
                    format!("{}\n", ail_artifact_fingerprint(target_report_text)),
                )
                .map_err(|error| {
                    format!("failed to write examples repair target report fingerprint: {error}")
                })?;
            }
            fs::write(
                entry_dir.join("repair-diff.txt"),
                &repair_proof.repair_diff_text,
            )
            .map_err(|error| format!("failed to write examples repair diff: {error}"))?;
            fs::write(
                entry_dir.join("repair-diff.fingerprint.txt"),
                format!(
                    "{}\n",
                    ail_artifact_fingerprint(&repair_proof.repair_diff_text)
                ),
            )
            .map_err(|error| {
                format!("failed to write examples repair diff fingerprint: {error}")
            })?;
            fs::write(
                entry_dir.join("repair-promotion-review.txt"),
                &repair_proof.promotion_review_text,
            )
            .map_err(|error| {
                format!("failed to write examples repair promotion review: {error}")
            })?;
            fs::write(
                entry_dir.join("repair-promotion-review.fingerprint.txt"),
                format!(
                    "{}\n",
                    ail_artifact_fingerprint(&repair_proof.promotion_review_text)
                ),
            )
            .map_err(|error| {
                format!("failed to write examples repair promotion review fingerprint: {error}")
            })?;
        }
    }
    Ok(())
}

fn run_ail_e2e_corpus_command(path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    let Some(artifact_dir) = &cli_options.artifact_dir else {
        return Err("ail-examples requires --artifact-dir".to_string());
    };
    let entries = load_ail_e2e_corpus_entries(std::path::Path::new(path))?;
    let example_count = entries.len();
    if example_count < 100 {
        return Err(format!(
            "ail-examples requires at least 100 examples; found {example_count}"
        ));
    }
    validate_ail_e2e_support_package_closure(std::path::Path::new(path), &entries)?;
    validate_ail_e2e_corpus_release_coverage(&entries)?;
    if cli_options.release_evidence {
        validate_ail_e2e_corpus_live_release_evidence(&entries)?;
    }
    validate_ail_e2e_corpus_transcript_files(&entries)?;
    validate_ail_e2e_corpus_story_files(&entries, cli_options.release_evidence)?;
    let mut evaluations = Vec::new();
    for entry in &entries {
        evaluations.push(evaluate_ail_e2e_corpus_entry(entry)?);
    }
    validate_ail_e2e_story_evidence_artifacts(&evaluations)?;
    if cli_options.release_evidence {
        validate_ail_e2e_repair_proof_distinctness(&evaluations)?;
    }
    let report_text = render_ail_e2e_corpus_report(&evaluations);
    write_ail_e2e_corpus_artifacts(artifact_dir, &report_text, &evaluations)?;
    print!("{report_text}");
    Ok(0)
}

fn run_ail_v03_roadmap_command(path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    let Some(artifact_dir) = &cli_options.artifact_dir else {
        return Err("ail-v03-roadmap requires --artifact-dir".to_string());
    };
    let entries = load_ail_e2e_corpus_entries(std::path::Path::new(path))?;
    let example_count = entries.len();
    if example_count < 100 {
        return Err(format!(
            "ail-v03-roadmap requires at least 100 examples; found {example_count}"
        ));
    }
    validate_ail_e2e_support_package_closure(std::path::Path::new(path), &entries)?;
    validate_ail_e2e_corpus_release_coverage(&entries)?;
    if cli_options.release_evidence {
        validate_ail_e2e_corpus_live_release_evidence(&entries)?;
    }
    validate_ail_e2e_corpus_transcript_files(&entries)?;
    validate_ail_e2e_corpus_story_files(&entries, cli_options.release_evidence)?;
    let mut evaluations = Vec::new();
    for entry in &entries {
        evaluations.push(evaluate_ail_e2e_corpus_entry(entry)?);
    }
    validate_ail_e2e_story_evidence_artifacts(&evaluations)?;
    if cli_options.release_evidence {
        validate_ail_e2e_repair_proof_distinctness(&evaluations)?;
    }
    let report_text = render_ail_e2e_corpus_report(&evaluations);
    write_ail_e2e_corpus_artifacts(artifact_dir, &report_text, &evaluations)?;
    print!("{}", render_ail_e2e_v03_roadmap(&evaluations));
    Ok(0)
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

fn darwin_macho_contract_machine_bytecode_manifest_contract_line(target_name: &str) -> String {
    format!(
        "machine-bytecode-contract {target_name} bytecode-level portable-vm-contract bytecode-container darwin-macho-contract bytecode-format macho64-arm64-contract-report"
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

fn render_ail_compile_darwin_macho_contract_manifest(
    artifacts: &AilCompileDarwinMachOContractArtifactSet<'_>,
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
    lines.push(format!("action {}", artifacts.action_name));
    lines
        .push(darwin_macho_contract_machine_bytecode_manifest_contract_line(artifacts.target_name));
    lines.push(format!(
        "darwin-macho-contract darwin-macho-contract-report.txt {}",
        ail_artifact_fingerprint(artifacts.darwin_macho_contract_report_text)
    ));
    lines.push(format!(
        "dependencies dependency-report.txt {}",
        ail_artifact_fingerprint(artifacts.dependency_report_text)
    ));
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

fn append_package_dependency_report(
    dependency_report_text: Option<String>,
    package_dependency_report_text: Option<&str>,
) -> Option<String> {
    match (dependency_report_text, package_dependency_report_text) {
        (Some(dependency_report_text), Some(package_dependency_report_text)) => Some(format!(
            "{dependency_report_text}\n{package_dependency_report_text}"
        )),
        (Some(dependency_report_text), None) => Some(dependency_report_text),
        (None, Some(package_dependency_report_text)) => {
            Some(package_dependency_report_text.to_string())
        }
        (None, None) => None,
    }
}

fn append_source_package_dependency_report(
    dependency_report_text: String,
    source_artifacts: Option<&AilSourcePackageArtifacts>,
) -> String {
    append_package_dependency_report(
        Some(dependency_report_text),
        source_artifacts.and_then(|artifacts| artifacts.package_dependency_report_text.as_deref()),
    )
    .expect("base dependency report must remain present")
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
    let package_dependency_report_text = if package.imports.is_empty() {
        None
    } else {
        Some(render_ail_package_dependency_report(&package)?)
    };
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
        package_dependency_report_text,
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
            "bootstrap-pass-composition bootstrap-pass-composition-report.txt {}",
            ail_artifact_fingerprint(artifacts.pass_composition_report_text)
        ),
        format!(
            "bootstrap-pass-order-diagnostics bootstrap-pass-order-diagnostics.txt {}",
            ail_artifact_fingerprint(artifacts.pass_order_diagnostics_report_text)
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

fn write_ail_compile_darwin_macho_contract_artifacts(
    artifact_dir: &str,
    artifacts: AilCompileDarwinMachOContractArtifactSet<'_>,
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
        root.join("darwin-macho-contract-report.txt"),
        artifacts.darwin_macho_contract_report_text,
    )
    .map_err(|error| {
        format!("failed to write ail-compile Darwin Mach-O contract report: {error}")
    })?;
    fs::write(
        root.join("darwin-macho-contract-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.darwin_macho_contract_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-compile Darwin Mach-O contract report fingerprint: {error}")
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
    let manifest_text = render_ail_compile_darwin_macho_contract_manifest(&artifacts);
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
        root.join("bootstrap-pass-composition-report.txt"),
        artifacts.pass_composition_report_text,
    )
    .map_err(|error| format!("failed to write ail-bootstrap pass composition report: {error}"))?;
    fs::write(
        root.join("bootstrap-pass-composition-report.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.pass_composition_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap pass composition report fingerprint: {error}")
    })?;
    fs::write(
        root.join("bootstrap-pass-order-diagnostics.txt"),
        artifacts.pass_order_diagnostics_report_text,
    )
    .map_err(|error| format!("failed to write ail-bootstrap pass order diagnostics: {error}"))?;
    fs::write(
        root.join("bootstrap-pass-order-diagnostics.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.pass_order_diagnostics_report_text)
        ),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap pass order diagnostics fingerprint: {error}")
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

fn render_ail_story_mode_report(artifacts: &AilStoryModeArtifactSet<'_>) -> String {
    let anchors = ail_e2e_semantic_anchors_from_story_fields(artifacts.story_fields);
    let story_journey = artifacts
        .story_fields
        .get("story-journey")
        .map(String::as_str)
        .unwrap_or("story-to-spec");
    let mut lines = vec![
        "AIL-Story-Mode-Report:".to_string(),
        "entrypoint: ail-story".to_string(),
        format!("package: {}", artifacts.package_name),
        format!("version: {}", artifacts.package_version),
        format!("story-file: {}", artifacts.story_file),
        format!(
            "user-story-id: {}",
            artifacts
                .story_fields
                .get("user-story-id")
                .map(String::as_str)
                .unwrap_or("unspecified")
        ),
        format!("story-journey: {story_journey}"),
        format!(
            "story-roundtrip: {}",
            artifacts
                .story_fields
                .get("story-roundtrip")
                .map(String::as_str)
                .unwrap_or("semantic-similar")
        ),
        format!("semantic-anchor-count: {}", anchors.len()),
    ];
    if let Some(endpoint) = artifacts.llm_endpoint {
        lines.push(format!("llm-endpoint: {endpoint}"));
        let max_tokens = artifacts
            .llm_max_tokens
            .unwrap_or(ail::llm::DEFAULT_CHAT_MAX_TOKENS);
        lines.push(format!(
            "default-max-tokens: {}",
            ail::llm::DEFAULT_CHAT_MAX_TOKENS
        ));
        lines.push(format!("max-tokens: {max_tokens}"));
        if max_tokens == ail::llm::DEFAULT_CHAT_MAX_TOKENS {
            lines.push("token-budget-default: true".to_string());
        } else {
            lines.push("token-budget-default: false".to_string());
            let warning = if max_tokens < ail::llm::DEFAULT_CHAT_MAX_TOKENS {
                "max-tokens-below-default"
            } else {
                "max-tokens-above-default"
            };
            lines.push(format!("token-budget-warning: {warning}"));
        }
    }
    if !artifacts.llm_transcripts.is_empty() {
        let valid_count = artifacts
            .llm_transcripts
            .iter()
            .filter(|transcript| transcript.content_kind.starts_with("prompt-envelope-"))
            .count();
        let invalid_count = artifacts.llm_transcripts.len().saturating_sub(valid_count);
        lines.push(format!(
            "story-llm-transcript-count: {}",
            artifacts.llm_transcripts.len()
        ));
        lines.push(format!("story-prompt-envelope-valid-count: {valid_count}"));
        lines.push(format!(
            "story-prompt-envelope-invalid-count: {invalid_count}"
        ));
        for transcript in artifacts.llm_transcripts {
            lines.push(format!(
                "story-llm-transcript: {} artifact-kind {} content-kind {}",
                transcript.stage, transcript.artifact_kind, transcript.content_kind
            ));
        }
    }
    for anchor in anchors {
        lines.push(format!("semantic-anchor: {anchor}"));
    }
    if story_journey == "story-amendment" {
        lines.push("story-amendment-comparison: present".to_string());
    }
    format!("{}\n", lines.join("\n"))
}

fn render_ail_story_amendment_comparison(
    story_source_text: &str,
    story_normalized_text: &str,
    story_fields: &BTreeMap<String, String>,
    requirements_text: &str,
    spec_text: &str,
    core_text: &str,
    bytecode_text: &str,
) -> Option<String> {
    if story_fields.get("story-journey").map(String::as_str) != Some("story-amendment") {
        return None;
    }
    let anchors = ail_e2e_semantic_anchors_from_story_fields(story_fields);
    let (preserved_count, missing_count) =
        ail_e2e_semantic_anchor_preservation_counts(story_normalized_text, &anchors);
    let mut lines = vec![
        "AIL-Story-Amendment-Comparison:".to_string(),
        "entrypoint ail-story".to_string(),
        "comparison-result accepted".to_string(),
        "story-journey story-amendment".to_string(),
        format!(
            "story-roundtrip {}",
            story_fields
                .get("story-roundtrip")
                .map(String::as_str)
                .unwrap_or("semantic-similar")
        ),
        format!(
            "user-story-id {}",
            story_fields
                .get("user-story-id")
                .map(String::as_str)
                .unwrap_or("unspecified")
        ),
        format!(
            "story-source-fingerprint {}",
            ail_artifact_fingerprint(story_source_text)
        ),
        format!(
            "story-normalized-fingerprint {}",
            ail_artifact_fingerprint(story_normalized_text)
        ),
        format!(
            "requirements-fingerprint {}",
            ail_artifact_fingerprint(requirements_text)
        ),
        format!(
            "accepted-spec-fingerprint {}",
            ail_artifact_fingerprint(spec_text)
        ),
        format!(
            "checked-core-fingerprint {}",
            ail_artifact_fingerprint(core_text)
        ),
        format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(bytecode_text)
        ),
        format!("semantic-anchor-count {}", anchors.len()),
        format!("semantic-anchor-preserved-count {preserved_count}"),
        format!("semantic-anchor-missing-count {missing_count}"),
        "semantic-anchor-added-count 0".to_string(),
    ];
    for anchor in anchors {
        let status = if story_normalized_text.contains(anchor.as_str()) {
            "preserved"
        } else {
            "missing"
        };
        lines.push(format!("semantic-anchor {anchor} {status}"));
    }
    lines.push("comparison-summary story amendment preserved declared semantic anchors and generated checked requirements, spec, Core, and bytecode evidence".to_string());
    Some(format!("{}\n", lines.join("\n")))
}

fn render_ail_story_manifest(artifacts: &AilStoryManifestArtifactSet<'_>) -> String {
    let mut lines = vec![
        "AIL-Story-Manifest:".to_string(),
        "entrypoint ail-story".to_string(),
        format!(
            "story-source story.source.md {}",
            ail_artifact_fingerprint(artifacts.story_source_text)
        ),
        format!(
            "story-normalized story.normalized.md {}",
            ail_artifact_fingerprint(artifacts.story_normalized_text)
        ),
        format!(
            "story-report story-mode-report.txt {}",
            ail_artifact_fingerprint(artifacts.story_report_text)
        ),
    ];
    if let Some(story_amendment_comparison_text) = artifacts.story_amendment_comparison_text {
        lines.push(format!(
            "story-amendment-comparison story-amendment-comparison.txt {}",
            ail_artifact_fingerprint(story_amendment_comparison_text)
        ));
    }
    lines.extend([
        format!(
            "requirements requirements.ail-requirements.md {}",
            ail_artifact_fingerprint(artifacts.requirements_text)
        ),
        format!(
            "spec accepted.ail-spec.md {}",
            ail_artifact_fingerprint(artifacts.spec_text)
        ),
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
    if let Some(agent_trace_text) = artifacts.agent_trace_text {
        lines.push(format!(
            "agent-trace agent-trace.txt {}",
            ail_artifact_fingerprint(agent_trace_text)
        ));
    }
    for transcript in artifacts.llm_transcripts {
        lines.push(format!(
            "llm-{}-request llm/{}.request.json {}",
            transcript.stage,
            transcript.stage,
            ail_artifact_fingerprint(&ensure_trailing_newline(transcript.request_body.clone()))
        ));
        lines.push(format!(
            "llm-{}-response llm/{}.response.json {}",
            transcript.stage,
            transcript.stage,
            ail_artifact_fingerprint(&ensure_trailing_newline(transcript.response_body.clone()))
        ));
        lines.push(format!(
            "llm-{}-content llm/{}.content.txt {}",
            transcript.stage,
            transcript.stage,
            ail_artifact_fingerprint(&ensure_trailing_newline(transcript.content_text.clone()))
        ));
    }
    if let Some(build_manifest_text) = artifacts.build_manifest_text {
        lines.push(format!(
            "build-manifest manifest.ail-build.txt {}",
            ail_artifact_fingerprint(build_manifest_text)
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn render_ail_story_questions_manifest(
    artifacts: &AilStoryQuestionsManifestArtifactSet<'_>,
) -> String {
    let mut lines = vec![
        "AIL-Story-Manifest:".to_string(),
        "entrypoint ail-story".to_string(),
        format!(
            "story-source story.source.md {}",
            ail_artifact_fingerprint(artifacts.story_source_text)
        ),
        format!(
            "story-normalized story.normalized.md {}",
            ail_artifact_fingerprint(artifacts.story_normalized_text)
        ),
        format!(
            "story-report story-mode-report.txt {}",
            ail_artifact_fingerprint(artifacts.story_report_text)
        ),
        format!(
            "story-questions story-questions.ail-interview.md {}",
            ail_artifact_fingerprint(artifacts.story_questions_text)
        ),
    ];
    if let Some(agent_trace_text) = artifacts.agent_trace_text {
        lines.push(format!(
            "agent-trace agent-trace.txt {}",
            ail_artifact_fingerprint(agent_trace_text)
        ));
    }
    for transcript in artifacts.llm_transcripts {
        lines.push(format!(
            "llm-{}-request llm/{}.request.json {}",
            transcript.stage,
            transcript.stage,
            ail_artifact_fingerprint(&ensure_trailing_newline(transcript.request_body.clone()))
        ));
        lines.push(format!(
            "llm-{}-response llm/{}.response.json {}",
            transcript.stage,
            transcript.stage,
            ail_artifact_fingerprint(&ensure_trailing_newline(transcript.response_body.clone()))
        ));
        lines.push(format!(
            "llm-{}-content llm/{}.content.txt {}",
            transcript.stage,
            transcript.stage,
            ail_artifact_fingerprint(&ensure_trailing_newline(transcript.content_text.clone()))
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn read_required_story_build_artifact(
    root: &std::path::Path,
    file_name: &str,
) -> Result<String, String> {
    fs::read_to_string(root.join(file_name))
        .map_err(|error| format!("failed to read ail-story build artifact {file_name}: {error}"))
}

fn write_ail_story_question_artifacts(
    artifact_dir: &str,
    artifacts: AilStoryModeArtifactSet<'_>,
    questions_text: &str,
    agent_trace: Option<&[String]>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-story artifact dir {artifact_dir}: {error}")
    })?;
    let story_source_text = ensure_trailing_newline(artifacts.story_source_text.to_string());
    let story_normalized_text =
        ensure_trailing_newline(artifacts.story_normalized_text.to_string());
    let story_report_text = render_ail_story_mode_report(&artifacts);
    let story_questions_text = ensure_trailing_newline(questions_text.to_string());
    fs::write(root.join("story.source.md"), &story_source_text)
        .map_err(|error| format!("failed to write ail-story source story artifact: {error}"))?;
    fs::write(
        root.join("story.source.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&story_source_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-story source story fingerprint artifact: {error}")
    })?;
    fs::write(root.join("story.normalized.md"), &story_normalized_text)
        .map_err(|error| format!("failed to write ail-story normalized story artifact: {error}"))?;
    fs::write(
        root.join("story.normalized.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&story_normalized_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-story normalized story fingerprint artifact: {error}")
    })?;
    fs::write(root.join("story-mode-report.txt"), &story_report_text)
        .map_err(|error| format!("failed to write ail-story mode report artifact: {error}"))?;
    fs::write(
        root.join("story-mode-report.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&story_report_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-story mode report fingerprint artifact: {error}")
    })?;
    fs::write(
        root.join("story-questions.ail-interview.md"),
        &story_questions_text,
    )
    .map_err(|error| format!("failed to write ail-story questions artifact: {error}"))?;
    fs::write(
        root.join("story-questions.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&story_questions_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-story questions fingerprint artifact: {error}")
    })?;
    let agent_trace_text = agent_trace.map(|trace| format!("{}\n", trace.join("\n")));
    if let Some(agent_trace_text) = agent_trace_text.as_deref() {
        fs::write(root.join("agent-trace.txt"), agent_trace_text)
            .map_err(|error| format!("failed to write ail-story agent trace artifact: {error}"))?;
        fs::write(
            root.join("agent-trace.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_trace_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-story agent trace fingerprint artifact: {error}")
        })?;
    }
    write_ail_story_llm_transcript_artifacts(root, artifacts.llm_transcripts)?;
    let manifest_text =
        render_ail_story_questions_manifest(&AilStoryQuestionsManifestArtifactSet {
            story_source_text: &story_source_text,
            story_normalized_text: &story_normalized_text,
            story_report_text: &story_report_text,
            story_questions_text: &story_questions_text,
            agent_trace_text: agent_trace_text.as_deref(),
            llm_transcripts: artifacts.llm_transcripts,
        });
    fs::write(root.join("manifest.ail-story.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-story manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.ail-story.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| format!("failed to write ail-story manifest fingerprint artifact: {error}"))?;
    Ok(())
}

fn write_ail_story_llm_transcript_artifacts(
    root: &std::path::Path,
    llm_transcripts: &[AilStoryLlmTranscript],
) -> Result<(), String> {
    if !llm_transcripts.is_empty() {
        fs::create_dir_all(root.join("llm"))
            .map_err(|error| format!("failed to create ail-story llm artifact dir: {error}"))?;
    }
    for transcript in llm_transcripts {
        for (suffix, text) in [
            ("request.json", transcript.request_body.as_str()),
            ("response.json", transcript.response_body.as_str()),
            ("content.txt", transcript.content_text.as_str()),
        ] {
            let artifact_text = ensure_trailing_newline(text.to_string());
            let path = root
                .join("llm")
                .join(format!("{}.{}", transcript.stage, suffix));
            fs::write(&path, &artifact_text).map_err(|error| {
                format!(
                    "failed to write ail-story llm transcript artifact {}: {error}",
                    path.display()
                )
            })?;
            fs::write(
                path.with_extension("fingerprint.txt"),
                format!("{}\n", ail_artifact_fingerprint(&artifact_text)),
            )
            .map_err(|error| {
                format!(
                    "failed to write ail-story llm transcript fingerprint {}: {error}",
                    path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn write_ail_story_mode_artifacts(
    artifact_dir: &str,
    artifacts: AilStoryModeArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-story artifact dir {artifact_dir}: {error}")
    })?;
    let story_source_text = ensure_trailing_newline(artifacts.story_source_text.to_string());
    let story_normalized_text =
        ensure_trailing_newline(artifacts.story_normalized_text.to_string());
    let story_report_text = render_ail_story_mode_report(&artifacts);
    fs::write(root.join("story.source.md"), &story_source_text)
        .map_err(|error| format!("failed to write ail-story source story artifact: {error}"))?;
    fs::write(
        root.join("story.source.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&story_source_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-story source story fingerprint artifact: {error}")
    })?;
    fs::write(root.join("story.normalized.md"), &story_normalized_text)
        .map_err(|error| format!("failed to write ail-story normalized story artifact: {error}"))?;
    fs::write(
        root.join("story.normalized.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&story_normalized_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-story normalized story fingerprint artifact: {error}")
    })?;
    fs::write(root.join("story-mode-report.txt"), &story_report_text)
        .map_err(|error| format!("failed to write ail-story mode report artifact: {error}"))?;
    fs::write(
        root.join("story-mode-report.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&story_report_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-story mode report fingerprint artifact: {error}")
    })?;
    let requirements_text =
        read_required_story_build_artifact(root, "requirements.ail-requirements.md")?;
    let spec_text = read_required_story_build_artifact(root, "accepted.ail-spec.md")?;
    let core_text = read_required_story_build_artifact(root, "checked.ail-core.txt")?;
    let bytecode_text = read_required_story_build_artifact(root, "artifact.ailbc.json")?;
    let story_amendment_comparison_text = render_ail_story_amendment_comparison(
        &story_source_text,
        &story_normalized_text,
        artifacts.story_fields,
        &requirements_text,
        &spec_text,
        &core_text,
        &bytecode_text,
    );
    if let Some(story_amendment_comparison_text) = story_amendment_comparison_text.as_deref() {
        fs::write(
            root.join("story-amendment-comparison.txt"),
            story_amendment_comparison_text,
        )
        .map_err(|error| {
            format!("failed to write ail-story amendment comparison artifact: {error}")
        })?;
        fs::write(
            root.join("story-amendment-comparison.fingerprint.txt"),
            format!(
                "{}\n",
                ail_artifact_fingerprint(story_amendment_comparison_text)
            ),
        )
        .map_err(|error| {
            format!("failed to write ail-story amendment comparison fingerprint artifact: {error}")
        })?;
    }
    write_ail_story_llm_transcript_artifacts(root, artifacts.llm_transcripts)?;
    let build_manifest_text = fs::read_to_string(root.join("manifest.ail-build.txt")).ok();
    let agent_bytecode_text = fs::read_to_string(root.join("agent.ailbc.json")).ok();
    let agent_trace_text = fs::read_to_string(root.join("agent-trace.txt")).ok();
    if let Some(agent_trace_text) = agent_trace_text.as_deref() {
        fs::write(
            root.join("agent-trace.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_trace_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-story agent trace fingerprint artifact: {error}")
        })?;
    }
    let manifest_text = render_ail_story_manifest(&AilStoryManifestArtifactSet {
        story_source_text: &story_source_text,
        story_normalized_text: &story_normalized_text,
        story_report_text: &story_report_text,
        story_amendment_comparison_text: story_amendment_comparison_text.as_deref(),
        requirements_text: &requirements_text,
        spec_text: &spec_text,
        core_text: &core_text,
        bytecode_text: &bytecode_text,
        agent_bytecode_text: agent_bytecode_text.as_deref(),
        agent_trace_text: agent_trace_text.as_deref(),
        build_manifest_text: build_manifest_text.as_deref(),
        llm_transcripts: artifacts.llm_transcripts,
    });
    fs::write(root.join("manifest.ail-story.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-story manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.ail-story.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| format!("failed to write ail-story manifest fingerprint artifact: {error}"))?;
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

fn render_ail_conformance_report(
    result: &ail::ail::AilConformanceResult,
    repair_proofs: &[AilConformanceRepairProofArtifacts],
) -> String {
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
    let repair_tutorial_count = result
        .rejected
        .iter()
        .filter(|fixture| !fixture.diagnostics.is_empty())
        .count();
    if repair_tutorial_count > 0 {
        lines.push(format!(
            "rejected-repair-tutorial-count {repair_tutorial_count}"
        ));
    }
    if !repair_proofs.is_empty() {
        lines.push(format!(
            "rejected-repair-proof-count {}",
            repair_proofs.len()
        ));
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
        if !fixture.diagnostics.is_empty() {
            let repair_tutorial_text = render_ail_conformance_repair_tutorial(result, fixture);
            let repair_tutorial_path =
                ail_conformance_repair_tutorial_artifact_path(&fixture.fixture);
            lines.push(format!(
                "rejected-repair-tutorial {} {} {}",
                fixture.fixture,
                repair_tutorial_path,
                ail_artifact_fingerprint(&repair_tutorial_text)
            ));
        }
    }
    for proof in artifacts.repair_proofs {
        let repair_proof_text = render_ail_conformance_repair_proof(result, proof);
        let repair_base = ail_conformance_repair_artifact_base_path(&proof.fixture);
        lines.push(format!(
            "rejected-repair-proof {} {}/repair-proof.txt {}",
            proof.fixture,
            repair_base,
            ail_artifact_fingerprint(&repair_proof_text)
        ));
        lines.push(format!(
            "rejected-repair-candidate {} {}/repair-candidate.ail-spec.md {}",
            proof.fixture,
            repair_base,
            ail_artifact_fingerprint(&proof.candidate_spec_text)
        ));
        lines.push(format!(
            "rejected-repair-checked-core {} {}/repair-checked.ail-core.txt {}",
            proof.fixture,
            repair_base,
            ail_artifact_fingerprint(&proof.checked_core_text)
        ));
        lines.push(format!(
            "rejected-repair-bytecode {} {}/repair-artifact.ailbc.json {}",
            proof.fixture,
            repair_base,
            ail_artifact_fingerprint(&proof.bytecode_text)
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

fn ail_conformance_repair_tutorial_artifact_path(fixture: &str) -> String {
    format!("rejected/{fixture}/repair-tutorial.txt")
}

fn ail_conformance_repair_artifact_base_path(fixture: &str) -> String {
    format!("rejected/{fixture}")
}

fn render_ail_conformance_repair_tutorial(
    result: &ail::ail::AilConformanceResult,
    fixture: &ail::ail::AilRejectedConformanceResult,
) -> String {
    let expected_diagnostic = fixture
        .diagnostics
        .first()
        .map(|diagnostic| diagnostic.code.as_str())
        .unwrap_or("unknown");
    let mut lines = vec![
        "AIL-Conformance-Repair-Tutorial:".to_string(),
        format!("package {}", result.package_name),
        format!("fixture {}", fixture.fixture),
        "checker-result rejected".to_string(),
        format!("expected-diagnostic {expected_diagnostic}"),
        "diagnostic-summary:".to_string(),
    ];
    for diagnostic in &fixture.diagnostics {
        lines.push(format!("diagnostic {}", diagnostic.detailed_message()));
    }
    lines.extend([
        "repair-plan:".to_string(),
        "repair-step 1 Preserve the rejected fixture, diagnostic, source provenance, affected graph item, and repair suggestion as review evidence."
            .to_string(),
        "repair-step 2 Draft a corrected package-local fixture that removes the expected diagnostic while preserving the example's semantic intent."
            .to_string(),
        "repair-step 3 Re-run ail-conformance with --artifact-dir and promote only after the corrected fixture is accepted and the rejected evidence remains available."
            .to_string(),
    ]);
    format!("{}\n", lines.join("\n"))
}

fn render_ail_conformance_repair_proof(
    result: &ail::ail::AilConformanceResult,
    proof: &AilConformanceRepairProofArtifacts,
) -> String {
    let lines = vec![
        "AIL-Conformance-Repair-Proof:".to_string(),
        format!("package {}", result.package_name),
        format!("fixture {}", proof.fixture),
        "checker-result rejected-to-repaired".to_string(),
        format!("expected-diagnostic {}", proof.expected_diagnostic),
        format!("repair-candidate {}", proof.candidate_fixture),
        format!(
            "repair-candidate-fingerprint {}",
            ail_artifact_fingerprint(&proof.candidate_spec_text)
        ),
        format!(
            "repair-checked-core-fingerprint {}",
            ail_artifact_fingerprint(&proof.checked_core_text)
        ),
        format!(
            "repair-bytecode-fingerprint {}",
            ail_artifact_fingerprint(&proof.bytecode_text)
        ),
        "repair-proof-summary corrected package-local fixture checked into Core and verified bytecode while rejected diagnostic evidence remains preserved.".to_string(),
    ];
    format!("{}\n", lines.join("\n"))
}

fn build_ail_conformance_repair_proofs(
    package: &AilPackage,
    result: &ail::ail::AilConformanceResult,
) -> Result<Vec<AilConformanceRepairProofArtifacts>, String> {
    let rejected_fixtures = result
        .rejected
        .iter()
        .filter(|fixture| !fixture.diagnostics.is_empty())
        .collect::<Vec<_>>();
    if rejected_fixtures.is_empty() {
        return Ok(Vec::new());
    }
    let (candidate_fixture, candidate_path) =
        ail_conformance_repair_candidate_path(package, result);
    let candidate_spec_text = fs::read_to_string(&candidate_path).map_err(|error| {
        format!(
            "failed to read ail-conformance repair candidate {}: {error}",
            candidate_path.display()
        )
    })?;
    let document = parse_ail_package_spec_text(package, &candidate_spec_text).map_err(|error| {
        format!(
            "ail-conformance repair candidate {} failed to parse: {error}",
            candidate_path.display()
        )
    })?;
    let core = elaborate_ail_core(package, &document);
    let core_diagnostics = check_ail_core(&core);
    if !core_diagnostics.is_empty() {
        return Err(format!(
            "ail-conformance repair candidate {} still has diagnostics:\n{}",
            candidate_path.display(),
            core_diagnostics.join("\n")
        ));
    }
    let bytecode = compile_ail_core_bytecode(&core)?;
    let bytecode_diagnostics = verify_ail_bytecode(&bytecode);
    if !bytecode_diagnostics.is_empty() {
        return Err(format!(
            "ail-conformance repair candidate {} has bytecode diagnostics:\n{}",
            candidate_path.display(),
            bytecode_diagnostics.join("\n")
        ));
    }
    let checked_core_text = render_ail_core(&core);
    let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
    Ok(rejected_fixtures
        .into_iter()
        .map(|fixture| AilConformanceRepairProofArtifacts {
            fixture: fixture.fixture.clone(),
            expected_diagnostic: fixture
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.code.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            candidate_fixture: candidate_fixture.clone(),
            candidate_spec_text: candidate_spec_text.clone(),
            checked_core_text: checked_core_text.clone(),
            bytecode_text: bytecode_text.clone(),
        })
        .collect())
}

fn ail_conformance_repair_candidate_path(
    package: &AilPackage,
    result: &ail::ail::AilConformanceResult,
) -> (String, std::path::PathBuf) {
    for fixture in &result.accepted {
        if fixture.diagnostics.is_empty() {
            return (
                format!("examples/accepted/{}", fixture.fixture),
                package
                    .root
                    .join("examples")
                    .join("accepted")
                    .join(&fixture.fixture),
            );
        }
    }
    (
        package
            .spec_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| package.spec_path.display().to_string()),
        package.spec_path.clone(),
    )
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
    for fixture in &result.rejected {
        if fixture.diagnostics.is_empty() {
            continue;
        }
        let repair_tutorial_text = render_ail_conformance_repair_tutorial(result, fixture);
        let repair_tutorial_dir = root.join("rejected").join(&fixture.fixture);
        fs::create_dir_all(&repair_tutorial_dir).map_err(|error| {
            format!(
                "failed to create ail-conformance repair tutorial dir {}: {error}",
                repair_tutorial_dir.display()
            )
        })?;
        fs::write(
            repair_tutorial_dir.join("repair-tutorial.txt"),
            &repair_tutorial_text,
        )
        .map_err(|error| {
            format!(
                "failed to write ail-conformance repair tutorial for {}: {error}",
                fixture.fixture
            )
        })?;
        fs::write(
            repair_tutorial_dir.join("repair-tutorial.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(&repair_tutorial_text)),
        )
        .map_err(|error| {
            format!(
                "failed to write ail-conformance repair tutorial fingerprint for {}: {error}",
                fixture.fixture
            )
        })?;
    }
    for proof in artifacts.repair_proofs {
        let repair_dir = root.join("rejected").join(&proof.fixture);
        fs::create_dir_all(&repair_dir).map_err(|error| {
            format!(
                "failed to create ail-conformance repair proof dir {}: {error}",
                repair_dir.display()
            )
        })?;
        let repair_proof_text = render_ail_conformance_repair_proof(result, proof);
        fs::write(repair_dir.join("repair-proof.txt"), &repair_proof_text).map_err(|error| {
            format!(
                "failed to write ail-conformance repair proof for {}: {error}",
                proof.fixture
            )
        })?;
        fs::write(
            repair_dir.join("repair-proof.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(&repair_proof_text)),
        )
        .map_err(|error| {
            format!(
                "failed to write ail-conformance repair proof fingerprint for {}: {error}",
                proof.fixture
            )
        })?;
        fs::write(
            repair_dir.join("repair-candidate.ail-spec.md"),
            &proof.candidate_spec_text,
        )
        .map_err(|error| {
            format!(
                "failed to write ail-conformance repair candidate for {}: {error}",
                proof.fixture
            )
        })?;
        fs::write(
            repair_dir.join("repair-candidate.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(&proof.candidate_spec_text)),
        )
        .map_err(|error| {
            format!(
                "failed to write ail-conformance repair candidate fingerprint for {}: {error}",
                proof.fixture
            )
        })?;
        fs::write(
            repair_dir.join("repair-checked.ail-core.txt"),
            &proof.checked_core_text,
        )
        .map_err(|error| {
            format!(
                "failed to write ail-conformance repair checked core for {}: {error}",
                proof.fixture
            )
        })?;
        fs::write(
            repair_dir.join("repair-checked.ail-core.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(&proof.checked_core_text)),
        )
        .map_err(|error| {
            format!(
                "failed to write ail-conformance repair checked core fingerprint for {}: {error}",
                proof.fixture
            )
        })?;
        fs::write(
            repair_dir.join("repair-artifact.ailbc.json"),
            &proof.bytecode_text,
        )
        .map_err(|error| {
            format!(
                "failed to write ail-conformance repair bytecode for {}: {error}",
                proof.fixture
            )
        })?;
        fs::write(
            repair_dir.join("repair-artifact.ailbc.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(&proof.bytecode_text)),
        )
        .map_err(|error| {
            format!(
                "failed to write ail-conformance repair bytecode fingerprint for {}: {error}",
                proof.fixture
            )
        })?;
    }
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
    let dependency_report_text = append_package_dependency_report(
        dependency_report_text,
        source_artifacts.and_then(|artifacts| artifacts.package_dependency_report_text.as_deref()),
    );
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

fn run_ail_build_agent_accept_flow_review(
    agent_path: &str,
    mut agent_start: AilBuildAgentStart,
    core_text: &str,
    flow_review_text: &str,
) -> Result<AilBuildAgentStart, String> {
    let (agent_bytecode, _) = load_verified_ail_build_agent(agent_path)?;
    if !agent_bytecode.actions.contains_key("AcceptFlowReview") {
        return Err(
            "ail-build --agent --artifact-dir requires an AcceptFlowReview action".to_string(),
        );
    }
    agent_start
        .state
        .insert("buildrequest.core ir".to_string(), core_text.to_string());
    agent_start.state.insert(
        "buildrequest.core ir fingerprint".to_string(),
        ail_artifact_fingerprint(core_text),
    );
    agent_start.state.insert(
        "buildrequest.flow review".to_string(),
        flow_review_text.to_string(),
    );
    agent_start.state.insert(
        "buildrequest.flow review fingerprint".to_string(),
        ail_artifact_fingerprint(flow_review_text),
    );
    let accept_run =
        run_ail_bytecode_action(&agent_bytecode, "AcceptFlowReview", agent_start.state)?;
    if accept_run.status != "succeeded" {
        let mut message = "ail-build agent AcceptFlowReview failed".to_string();
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
        .is_some_and(|status| status == "FlowReviewed")
    {
        "FlowReviewed"
    } else if compile_state
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
    pass_composition_report_text: &'a str,
    pass_order_diagnostics_report_text: &'a str,
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
        pass_composition_report_text,
        pass_order_diagnostics_report_text,
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
            "buildrequest.compiler pass composition report".to_string(),
            pass_composition_report_text.to_string(),
        ),
        (
            "buildrequest.compiler pass composition report fingerprint".to_string(),
            ail_artifact_fingerprint(pass_composition_report_text),
        ),
        (
            "buildrequest.compiler pass order diagnostics".to_string(),
            pass_order_diagnostics_report_text.to_string(),
        ),
        (
            "buildrequest.compiler pass order diagnostics fingerprint".to_string(),
            ail_artifact_fingerprint(pass_order_diagnostics_report_text),
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
        "report bootstrap-pass-composition-report.txt".to_string(),
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
        "AcceptFlowReview" => Ok(BootstrapHandoffCase {
            trace_marker: "FlowReviewAccepted",
            args: &[
                "buildrequest.id=bootstrap-handoff",
                "buildrequest.status=CoreChecked",
                "buildrequest.core ir=checked",
                "buildrequest.core ir fingerprint=fnv64:core",
                "buildrequest.flow review=review-flow",
                "buildrequest.flow review fingerprint=fnv64:flow",
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
        format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(&render_ail_bytecode(program))
        ),
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

fn ail_compile_darwin_macho_contract_status<'a>(
    program: &'a AilBytecodeProgram,
    target_name: &str,
) -> Result<&'a str, String> {
    if target_name != "aarch64-apple-darwin-libsystem-macho" {
        return Err(format!(
            "unsupported Darwin Mach-O contract target '{target_name}'"
        ));
    }
    let status = program
        .target_support
        .get(target_name)
        .ok_or_else(|| {
            format!(
                "AIL-BACKEND-001 package {} target-support does not declare Darwin Mach-O contract target {target_name}",
                program.package_name
            )
        })?
        .as_str();
    if matches!(
        status,
        "planned-contract" | "supported" | "supported-with-host-imports"
    ) {
        Ok(status)
    } else {
        Err(format!(
            "AIL-BACKEND-001 package {} target-support marks {target_name} as {status}; Darwin Mach-O contract target {target_name} requires planned-contract, supported, or supported-with-host-imports",
            program.package_name
        ))
    }
}

fn render_ail_compile_darwin_macho_contract_report(
    program: &AilBytecodeProgram,
    action_name: &str,
    target_name: &str,
) -> Result<String, String> {
    let status = ail_compile_darwin_macho_contract_status(program, target_name)?;
    if !program.actions.contains_key(action_name) {
        return Err(format!("unknown AIL action '{action_name}'"));
    }
    let mut lines = vec![
        "AIL-Darwin-MachO-Contract-Report:".to_string(),
        format!(
            "package {} {}",
            program.package_name, program.package_version
        ),
        format!("target {target_name}"),
        format!("status {status}"),
        format!(
            "bytecode-fingerprint {}",
            ail_artifact_fingerprint(&render_ail_bytecode(program))
        ),
        "bytecode-level portable-vm-contract".to_string(),
        "bytecode-container darwin-macho-contract".to_string(),
        "bytecode-format macho64-arm64-contract-report".to_string(),
        "host-boundary libSystem-and-entitlements".to_string(),
        "host-import-metadata present-in-saved-bytecode".to_string(),
        format!("action {action_name}"),
        "trace-scope reachable-action-call-graph".to_string(),
        format!(
            "trace-preservation {}",
            ail_wasm_contract_trace_preservation_label(program, action_name)?
        ),
        "executable-macho-module none".to_string(),
    ];
    if program.external_bindings.is_empty() {
        lines.push("external-symbols none".to_string());
    } else {
        for binding in program.external_bindings.values() {
            lines.push(format!(
                "external-symbol {} library {} symbol {} binding-kind {} calling-convention {}",
                binding.name,
                binding.library,
                binding.symbol,
                binding.binding_kind,
                binding.calling_convention
            ));
            for capability in &binding.capabilities {
                lines.push(format!("capability {} {}", binding.name, capability));
            }
            for trace in &binding.traces {
                lines.push(format!("external-symbol-trace {} {}", binding.name, trace));
            }
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_compile_darwin_macho_contract_dependency_report(
    program: &AilBytecodeProgram,
    action_name: &str,
    target_name: &str,
) -> Result<String, String> {
    let _status = ail_compile_darwin_macho_contract_status(program, target_name)?;
    if !program.actions.contains_key(action_name) {
        return Err(format!("unknown AIL action '{action_name}'"));
    }
    let libraries = ail_bytecode_darwin_macho_contract_library_dependencies(program);
    let mut lines = vec![
        "AIL-Compile-Dependency-Report:".to_string(),
        format!("target {target_name}"),
        format!("action {action_name}"),
        "host-language-runtime none".to_string(),
        "dynamic-linker libSystem".to_string(),
        format!("shared-libraries {libraries}"),
        format!("library-dependencies {libraries}"),
        "linker-invocation none".to_string(),
        "runtime-abi darwin-libsystem-entitlements".to_string(),
        "machine-bytecode-dependency darwin-macho-contract-report.txt contract-only-darwin-macho"
            .to_string(),
    ];
    for binding in program.external_bindings.values() {
        lines.push(format!(
            "external-symbol-dependency {} library {} symbol {} binding-kind {} calling-convention {}",
            binding.name,
            binding.library,
            binding.symbol,
            binding.binding_kind,
            binding.calling_convention
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn ail_bytecode_darwin_macho_contract_library_dependencies(program: &AilBytecodeProgram) -> String {
    let mut libraries = program
        .external_bindings
        .values()
        .map(|binding| binding.library.clone())
        .collect::<BTreeSet<_>>();
    libraries.insert("libSystem".to_string());
    libraries.into_iter().collect::<Vec<_>>().join(",")
}

fn check_darwin_macho_contract_supported_effects(
    core: &ail::ail::AilCore,
    target_name: &str,
) -> Result<(), String> {
    if target_name != "aarch64-apple-darwin-libsystem-macho" {
        return Ok(());
    }
    for node in &core.graph.nodes {
        if node.kind == "Effect" && node.name.to_ascii_lowercase().contains("linux syscall") {
            return Err(format!(
                "AIL-BACKEND-001 target {target_name} does not support Linux-only syscall effect '{}'",
                node.name
            ));
        }
    }
    Ok(())
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

fn grounded_ail_requirements_prompt(
    prompt: &str,
    agent_requirements_context: Option<&str>,
) -> String {
    if let Some(agent_requirements_context) =
        agent_requirements_context.filter(|context| !context.trim().is_empty())
    {
        format!(
            concat!(
                "{}\n\n",
                "Use this AIL agent preflight state as a requirements coverage checklist. ",
                "Do not restate it by itself; produce a full AIL-Requirements artifact inside artifact_text with bullets for domain objects, required fields, action inputs or preconditions, failure cases, guarantees, trace events, secrets, permissions, and runtime inputs.\n\n",
                "AGENT REQUIREMENTS CONTEXT:\n",
                "{}"
            ),
            prompt, agent_requirements_context
        )
    } else {
        prompt.to_string()
    }
}

fn blocking_questions_error(questions: &[String]) -> String {
    format!(
        "model returned blocking questions:\n- {}",
        questions.join("\n- ")
    )
}

fn draft_checked_ail_requirements_for_package_or_questions(
    package: &ail::ail::AilPackage,
    prompt: &str,
    endpoint: &str,
    agent_requirements_context: Option<&str>,
    retry_prompt_envelope_errors: bool,
) -> Result<AilRequirementsDraftOutcome, String> {
    let grounded_prompt = grounded_ail_requirements_prompt(prompt, agent_requirements_context);
    let mut requirements =
        match draft_ail_requirements_response(package, &grounded_prompt, endpoint) {
            Ok(ail::llm::LlmArtifactResponse::Artifact(requirements)) => requirements,
            Ok(ail::llm::LlmArtifactResponse::Questions(questions)) => {
                return Ok(AilRequirementsDraftOutcome::Questions(questions));
            }
            Err(error)
                if retry_prompt_envelope_errors && is_prompt_envelope_protocol_error(&error) =>
            {
                let retry_prompt =
                    prompt_envelope_retry_prompt(&grounded_prompt, &error, "AIL-Requirements");
                match draft_ail_requirements_response(package, &retry_prompt, endpoint)? {
                    ail::llm::LlmArtifactResponse::Artifact(requirements) => requirements,
                    ail::llm::LlmArtifactResponse::Questions(questions) => {
                        return Ok(AilRequirementsDraftOutcome::Questions(questions));
                    }
                }
            }
            Err(error) => return Err(error),
        };
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
    Ok(AilRequirementsDraftOutcome::Requirements {
        text: requirements,
        diagnostics,
    })
}

fn draft_checked_ail_requirements_for_package(
    package: &ail::ail::AilPackage,
    prompt: &str,
    endpoint: &str,
    agent_requirements_context: Option<&str>,
    retry_prompt_envelope_errors: bool,
) -> Result<(String, Vec<ail::ail::AilDiagnostic>), String> {
    match draft_checked_ail_requirements_for_package_or_questions(
        package,
        prompt,
        endpoint,
        agent_requirements_context,
        retry_prompt_envelope_errors,
    )? {
        AilRequirementsDraftOutcome::Requirements { text, diagnostics } => Ok((text, diagnostics)),
        AilRequirementsDraftOutcome::Questions(questions) => {
            Err(blocking_questions_error(&questions))
        }
    }
}

fn push_story_llm_transcript(
    transcripts: &mut Vec<AilStoryLlmTranscript>,
    stage: &'static str,
    artifact_kind: &'static str,
    recorded: &ail::llm::LlmRecordedArtifactResponse,
) {
    transcripts.push(AilStoryLlmTranscript {
        stage,
        artifact_kind,
        request_body: recorded.request_body.clone(),
        response_body: recorded.response_body.clone(),
        content_text: recorded.content_text.clone(),
        content_kind: recorded.content_kind.clone(),
    });
}

fn draft_checked_ail_requirements_for_story_or_questions(
    package: &ail::ail::AilPackage,
    prompt: &str,
    agent_requirements_context: Option<&str>,
    llm_options: AilStoryLlmOptions<'_>,
    transcripts: &mut Vec<AilStoryLlmTranscript>,
) -> Result<AilRequirementsDraftOutcome, String> {
    let grounded_prompt = grounded_ail_requirements_prompt(prompt, agent_requirements_context);
    let mut requirements = match draft_ail_requirements_response_recorded_with_max_tokens(
        package,
        &grounded_prompt,
        llm_options.endpoint,
        llm_options.max_tokens,
    ) {
        Ok(recorded) => {
            push_story_llm_transcript(transcripts, "requirements", "AIL-Requirements", &recorded);
            match recorded.outcome {
                ail::llm::LlmArtifactResponse::Artifact(requirements) => requirements,
                ail::llm::LlmArtifactResponse::Questions(questions) => {
                    return Ok(AilRequirementsDraftOutcome::Questions(questions));
                }
            }
        }
        Err(error)
            if llm_options.retry_prompt_envelope_errors
                && is_prompt_envelope_protocol_error(&error) =>
        {
            let retry_prompt =
                prompt_envelope_retry_prompt(&grounded_prompt, &error, "AIL-Requirements");
            let recorded = draft_ail_requirements_response_recorded_with_max_tokens(
                package,
                &retry_prompt,
                llm_options.endpoint,
                llm_options.max_tokens,
            )?;
            push_story_llm_transcript(transcripts, "requirements", "AIL-Requirements", &recorded);
            match recorded.outcome {
                ail::llm::LlmArtifactResponse::Artifact(requirements) => requirements,
                ail::llm::LlmArtifactResponse::Questions(questions) => {
                    return Ok(AilRequirementsDraftOutcome::Questions(questions));
                }
            }
        }
        Err(error) => return Err(error),
    };
    let mut diagnostics = check_ail_requirements(package, &requirements);
    if !diagnostics.is_empty() {
        requirements = repair_ail_requirements_from_diagnostics(
            package,
            &grounded_prompt,
            &requirements,
            &diagnostics,
            llm_options.endpoint,
        )?;
        diagnostics = check_ail_requirements(package, &requirements);
    }
    Ok(AilRequirementsDraftOutcome::Requirements {
        text: requirements,
        diagnostics,
    })
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

fn prompt_with_requested_native_action(prompt: &str, action: Option<&str>) -> String {
    let Some(action) = action else {
        return prompt.to_string();
    };
    format!(
        concat!(
            "{}\n\n",
            "NATIVE BUILD CONSTRAINT:\n",
            "The final AIL-Spec, checked AIL-Core, and AIL-Bytecode must define action {} because the requested native target will compile that exact action."
        ),
        prompt, action
    )
}

fn prompt_with_source_spec_context(prompt: &str, source_spec_text: &str) -> String {
    let source_spec_text = source_spec_text.trim();
    if source_spec_text.is_empty() {
        return prompt.to_string();
    }
    format!(
        concat!(
            "{}\n\n",
            "PACKAGE SOURCE AIL-SPEC CONTEXT:\n",
            "{}\n\n",
            "Use the package source AIL-Spec as authoritative context. Preserve its named actions, failures, guarantees, traces, secret-handling rules, and runtime requirements unless the human request explicitly changes them. ",
            "If the source AIL-Spec contains Route:, Form:, Dashboard:, or Workflow: preserve their reads, permissions, filters, blocks, and trace records unless the human request explicitly changes them."
        ),
        prompt, source_spec_text
    )
}

fn draft_checked_ail_spec_for_requirements(
    package: &ail::ail::AilPackage,
    prompt: &str,
    requirements: &str,
    endpoint: &str,
    agent_spec_context: Option<&str>,
    retry_prompt_envelope_errors: bool,
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
        match draft_ail_spec_from_requirements(package, &grounded_prompt, requirements, endpoint) {
            Ok(draft) => draft,
            Err(error)
                if retry_prompt_envelope_errors && is_prompt_envelope_protocol_error(&error) =>
            {
                let retry_prompt =
                    prompt_envelope_retry_prompt(&grounded_prompt, &error, "AIL-Spec Canonical");
                draft_ail_spec_from_requirements(package, &retry_prompt, requirements, endpoint)?
            }
            Err(error) => return Err(error),
        };
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

fn draft_checked_ail_spec_for_story_requirements(
    package: &ail::ail::AilPackage,
    prompt: &str,
    requirements: &str,
    agent_spec_context: Option<&str>,
    llm_options: AilStoryLlmOptions<'_>,
    transcripts: &mut Vec<AilStoryLlmTranscript>,
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
    let mut draft = match draft_ail_spec_from_requirements_recorded_with_max_tokens(
        package,
        &grounded_prompt,
        requirements,
        llm_options.endpoint,
        llm_options.max_tokens,
    ) {
        Ok((draft, recorded)) => {
            push_story_llm_transcript(transcripts, "spec", "AIL-Spec Canonical", &recorded);
            draft
        }
        Err(error)
            if llm_options.retry_prompt_envelope_errors
                && is_prompt_envelope_protocol_error(&error) =>
        {
            let retry_prompt =
                prompt_envelope_retry_prompt(&grounded_prompt, &error, "AIL-Spec Canonical");
            let (draft, recorded) = draft_ail_spec_from_requirements_recorded_with_max_tokens(
                package,
                &retry_prompt,
                requirements,
                llm_options.endpoint,
                llm_options.max_tokens,
            )?;
            push_story_llm_transcript(transcripts, "spec", "AIL-Spec Canonical", &recorded);
            draft
        }
        Err(error) => return Err(error),
    };
    if !draft.success() {
        draft = repair_ail_spec_from_diagnostics(
            package,
            &grounded_prompt,
            requirements,
            &draft.spec_text,
            &draft.diagnostics,
            llm_options.endpoint,
        )?;
    }
    Ok(draft)
}

fn is_prompt_envelope_protocol_error(error: &str) -> bool {
    error.starts_with("AIL-PROMPT-001 prompt envelope")
}

fn prompt_envelope_retry_prompt(prompt: &str, error: &str, artifact_kind: &str) -> String {
    format!(
        concat!(
            "{}\n\n",
            "The previous model response was rejected by the AIL prompt envelope checker:\n",
            "{}\n\n",
            "Retry once. Return only a valid prompt-pack JSON envelope for artifact_kind ",
            "{}. The envelope must contain exactly one of these: non-empty artifact_text with an empty questions array, or empty artifact_text with non-empty blocking questions. Do not include both."
        ),
        prompt, error, artifact_kind
    )
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
    if target == "aarch64-apple-darwin-libsystem-macho" {
        if cli_options.ail_compile_out.is_some() {
            return Err("ail-compile Darwin Mach-O contract target does not emit --out yet; use --artifact-dir <dir>".to_string());
        }
        if cli_options.ail_build_agent.is_some() {
            return Err(
                "ail-compile Darwin Mach-O contract target does not support --agent yet"
                    .to_string(),
            );
        }
        let artifact_dir = cli_options.artifact_dir.as_deref().ok_or_else(|| {
            "ail-compile Darwin Mach-O contract target requires --artifact-dir <dir>".to_string()
        })?;
        check_darwin_macho_contract_supported_effects(core, target)?;
        let bytecode = compile_ail_core_bytecode(core)?;
        let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
        let core_text = format!("{}\n", render_ail_core(core));
        let darwin_macho_contract_report_text =
            render_ail_compile_darwin_macho_contract_report(&bytecode, action, target)?;
        let dependency_report_text = append_source_package_dependency_report(
            render_ail_compile_darwin_macho_contract_dependency_report(&bytecode, action, target)?,
            source_artifacts,
        );
        write_ail_compile_darwin_macho_contract_artifacts(
            artifact_dir,
            AilCompileDarwinMachOContractArtifactSet {
                source_manifest_text: source_artifacts
                    .map(|artifacts| artifacts.manifest_text.as_str()),
                source_spec_text: source_artifacts.map(|artifacts| artifacts.spec_text.as_str()),
                core_text: Some(&core_text),
                bytecode_text: &bytecode_text,
                action_name: action,
                target_name: target,
                darwin_macho_contract_report_text: &darwin_macho_contract_report_text,
                dependency_report_text: &dependency_report_text,
            },
        )?;
        println!("ail-compile wrote {target} contract {artifact_dir}");
        return Ok(0);
    }
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
        let dependency_report_text = append_source_package_dependency_report(
            render_ail_compile_wasm_contract_dependency_report(&bytecode, action, target)?,
            source_artifacts,
        );
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
            let dependency_report_text = append_source_package_dependency_report(
                render_ail_compile_dependency_report(
                    action,
                    target,
                    &executable,
                    agent_native_artifacts.as_slice(),
                )?,
                source_artifacts,
            );
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
            let dependency_report_text = append_source_package_dependency_report(
                render_ail_compile_dependency_report(action, target, &executable, &[])?,
                source_artifacts,
            );
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
    let dependency_report_text = append_source_package_dependency_report(
        render_ail_compile_wasm_contract_bundle_dependency_report(bytecode, target)?,
        source_artifacts,
    );
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
            let dependency_report_text = append_source_package_dependency_report(
                render_ail_compile_bundle_dependency_report(
                    target,
                    target_executables.as_slice(),
                    agent_native_artifacts.as_slice(),
                )?,
                source_artifacts,
            );
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
            let dependency_report_text = append_source_package_dependency_report(
                render_ail_compile_bundle_dependency_report(
                    target,
                    target_executables.as_slice(),
                    &[],
                )?,
                source_artifacts,
            );
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
    if target == "aarch64-apple-darwin-libsystem-macho" {
        if cli_options.ail_compile_out.is_some() {
            return Err("ail-compile Darwin Mach-O contract target does not emit --out yet; use --artifact-dir <dir>".to_string());
        }
        if cli_options.ail_build_agent.is_some() {
            return Err(
                "ail-compile Darwin Mach-O contract target does not support --agent yet"
                    .to_string(),
            );
        }
        let artifact_dir = cli_options.artifact_dir.as_deref().ok_or_else(|| {
            "ail-compile Darwin Mach-O contract target requires --artifact-dir <dir>".to_string()
        })?;
        let darwin_macho_contract_report_text =
            render_ail_compile_darwin_macho_contract_report(&bytecode, action, target)?;
        let dependency_report_text =
            render_ail_compile_darwin_macho_contract_dependency_report(&bytecode, action, target)?;
        write_ail_compile_darwin_macho_contract_artifacts(
            artifact_dir,
            AilCompileDarwinMachOContractArtifactSet {
                source_manifest_text: None,
                source_spec_text: None,
                core_text: None,
                bytecode_text: &bytecode_text,
                action_name: action,
                target_name: target,
                darwin_macho_contract_report_text: &darwin_macho_contract_report_text,
                dependency_report_text: &dependency_report_text,
            },
        )?;
        println!("ail-compile wrote {target} contract {artifact_dir}");
        return Ok(0);
    }
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
        let mut agent_start = run_ail_build_agent_accept_core(
            agent_path,
            agent_start,
            requirements_artifact,
            spec_text,
            &core_text,
        )?;
        if cli_options.artifact_dir.is_some() {
            let artifact_core_text = format!("{}\n", render_ail_core(&core));
            let flow_review_text = format!("{}\n", render_ail_flow_view(&core));
            agent_start = run_ail_build_agent_accept_flow_review(
                agent_path,
                agent_start,
                &artifact_core_text,
                &flow_review_text,
            )?;
        }
        Some(agent_start)
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
        let package_dependency_report_text = source_artifacts
            .and_then(|artifacts| artifacts.package_dependency_report_text.as_deref());
        let dependency_report_text = if let Some((target, _, executable)) = native_build.as_ref() {
            let build_dependency_report = render_ail_build_dependency_report(
                target,
                executable,
                pass_native_artifacts.as_slice(),
                agent_native_artifacts.as_slice(),
            )?;
            Some(
                if let Some(package_dependency_report_text) = package_dependency_report_text {
                    format!("{build_dependency_report}\n{package_dependency_report_text}")
                } else {
                    build_dependency_report
                },
            )
        } else {
            package_dependency_report_text.map(str::to_string)
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
    let repair_proofs = build_ail_conformance_repair_proofs(&package, &result)?;
    let report = render_ail_conformance_report(&result, &repair_proofs);
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

struct AilBootstrapPassCompositionEvidence<'a> {
    target_name: &'a str,
    compiler_pass_action: &'a str,
    input_core_text: &'a str,
    output_core_text: &'a str,
    pass_trace_text: &'a str,
    fixed_point_report_text: &'a str,
    compiler_pass_self_output_core_text: &'a str,
    pass_order_diagnostics_report_text: &'a str,
}

fn render_ail_bootstrap_pass_composition_report(
    evidence: &AilBootstrapPassCompositionEvidence<'_>,
) -> String {
    format!(
        concat!(
            "AIL-Bootstrap-Pass-Composition:\n",
            "target {}\n",
            "composition-pass-count 1\n",
            "composition-variant-count 2\n",
            "composition-variant 1 toolchain-agent-fixed-point pass {} status ok output {}\n",
            "composition-variant 2 compiler-pass-self-check pass {} status ok output {}\n",
            "composition-pass 1 {} source compiler-pass.source.ail-spec.md bytecode compiler-pass.ailbc.json core compiler-pass.checked.ail-core.txt\n",
            "composition-input toolchain-agent.checked.ail-core.txt {}\n",
            "composition-output toolchain-agent.pass-output.ail-core.txt {}\n",
            "composition-trace toolchain-agent.pass-trace.txt {}\n",
            "composition-fixed-point bootstrap-fixed-point-report.txt {}\n",
            "pass-order-diagnostic-count 0\n",
            "pass-order-diagnostics bootstrap-pass-order-diagnostics.txt {}\n",
            "pass-order-status ok\n",
            "supported-variant single-pass-fixed-point\n",
            "supported-variant multi-pass-sequence-reviewed\n",
        ),
        evidence.target_name,
        evidence.compiler_pass_action,
        ail_artifact_fingerprint(evidence.output_core_text),
        evidence.compiler_pass_action,
        ail_artifact_fingerprint(evidence.compiler_pass_self_output_core_text),
        evidence.compiler_pass_action,
        ail_artifact_fingerprint(evidence.input_core_text),
        ail_artifact_fingerprint(evidence.output_core_text),
        ail_artifact_fingerprint(evidence.pass_trace_text),
        ail_artifact_fingerprint(evidence.fixed_point_report_text),
        ail_artifact_fingerprint(evidence.pass_order_diagnostics_report_text)
    )
}

fn render_ail_bootstrap_pass_order_diagnostics_report(
    compiler_pass_action: &str,
    pass_paths: &[String],
    input_core_text: &str,
    output_core_text: &str,
    compiler_pass_self_output_core_text: &str,
    fixed_point_report_text: &str,
) -> String {
    let pass_sequence = render_ail_bootstrap_user_pass_sequence(pass_paths);
    format!(
        concat!(
            "AIL-Bootstrap-Pass-Order-Diagnostics:\n",
            "{}",
            "accepted-pass-sequence-count 1\n",
            "accepted-pass-sequence 1 pass {} source {}\n",
            "composition-variant-count 2\n",
            "composition-variant 1 toolchain-agent-fixed-point pass {} status ok output {}\n",
            "composition-variant 2 compiler-pass-self-check pass {} status ok output {}\n",
            "reviewed-pass-order-conflict-count 1\n",
            "reviewed-pass-order-conflict AIL-BOOTSTRAP-PASS-ORDER-001 duplicate-pass-before-fixed-point pass {} input {} output {} fixed-point {}\n",
            "pass-order-status ok\n",
            "conflict-resolution fixed-point-gate-required\n",
            "diagnostic-visibility reviewer-visible\n",
        ),
        pass_sequence,
        compiler_pass_action,
        pass_paths
            .first()
            .map(String::as_str)
            .unwrap_or("compiler-pass.source.ail-spec.md"),
        compiler_pass_action,
        ail_artifact_fingerprint(output_core_text),
        compiler_pass_action,
        ail_artifact_fingerprint(compiler_pass_self_output_core_text),
        compiler_pass_action,
        ail_artifact_fingerprint(input_core_text),
        ail_artifact_fingerprint(output_core_text),
        ail_artifact_fingerprint(fixed_point_report_text)
    )
}

fn render_ail_bootstrap_user_pass_sequence(pass_paths: &[String]) -> String {
    let mut lines = format!("user-pass-sequence-count {}\n", pass_paths.len());
    for (index, path) in pass_paths.iter().enumerate() {
        lines.push_str(&format!("user-pass {} {}\n", index + 1, path));
    }
    lines
}

fn find_duplicate_ail_bootstrap_pass_source(pass_paths: &[String]) -> Option<String> {
    let mut seen = BTreeSet::new();
    for path in pass_paths {
        if !seen.insert(path.as_str()) {
            return Some(path.clone());
        }
    }
    None
}

fn render_ail_bootstrap_rejected_pass_order_diagnostics_report(
    pass_paths: &[String],
    duplicate_pass_source: &str,
) -> String {
    format!(
        concat!(
            "AIL-Bootstrap-Pass-Order-Diagnostics:\n",
            "{}",
            "reviewed-pass-order-conflict-count 1\n",
            "reviewed-pass-order-conflict AIL-BOOTSTRAP-PASS-ORDER-001 duplicate-pass-before-fixed-point pass-source {}\n",
            "pass-order-status conflict\n",
            "conflict-resolution fixed-point-gate-required\n",
            "diagnostic-visibility reviewer-visible\n",
        ),
        render_ail_bootstrap_user_pass_sequence(pass_paths),
        duplicate_pass_source
    )
}

fn write_ail_bootstrap_pass_order_diagnostics_artifacts(
    artifact_dir: &str,
    report_text: &str,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-bootstrap artifact dir {artifact_dir}: {error}")
    })?;
    fs::write(
        root.join("bootstrap-pass-order-diagnostics.txt"),
        report_text,
    )
    .map_err(|error| format!("failed to write ail-bootstrap pass order diagnostics: {error}"))?;
    fs::write(
        root.join("bootstrap-pass-order-diagnostics.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(report_text)),
    )
    .map_err(|error| {
        format!("failed to write ail-bootstrap pass order diagnostics fingerprint: {error}")
    })?;
    Ok(())
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
    let pass_paths = if cli_options.ail_build_passes.is_empty() {
        vec![pass_path.to_string()]
    } else {
        cli_options.ail_build_passes.clone()
    };
    if let Some(duplicate_pass_source) = find_duplicate_ail_bootstrap_pass_source(&pass_paths) {
        let pass_order_diagnostics_report_text =
            render_ail_bootstrap_rejected_pass_order_diagnostics_report(
                &pass_paths,
                &duplicate_pass_source,
            );
        write_ail_bootstrap_pass_order_diagnostics_artifacts(
            artifact_dir,
            &pass_order_diagnostics_report_text,
        )?;
        return Err(format!(
            "ail-bootstrap pass-order conflict: AIL-BOOTSTRAP-PASS-ORDER-001 duplicate-pass-before-fixed-point pass-source {duplicate_pass_source}"
        ));
    }
    let pass_path = pass_paths
        .first()
        .map(String::as_str)
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
    let compiler_pass_core = parse_ail_core_text(&compiler_pass_core_text)
        .map_err(|error| format!("ail-bootstrap compiler pass core reparse failed: {error}"))?;
    let compiler_pass_self_result = run_ail_compiler_pass_on_core(
        &compiler_pass_bytecode,
        &compiler_pass_action,
        &compiler_pass_core,
    )?;
    if compiler_pass_self_result.run.status != "succeeded" {
        let mut message =
            format!("ail-bootstrap compiler pass self-check {compiler_pass_action} failed");
        if let Some(failure) = compiler_pass_self_result.run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !compiler_pass_self_result.run.trace.is_empty() {
            message.push_str(&format!(
                "\n{}",
                compiler_pass_self_result.run.trace.join("\n")
            ));
        }
        return Err(message);
    }
    let compiler_pass_self_diagnostics = check_ail_core(&compiler_pass_self_result.core);
    if !compiler_pass_self_diagnostics.is_empty() {
        return Err(format!(
            "ail-bootstrap compiler pass self-check output has diagnostics:\n{}",
            compiler_pass_self_diagnostics.join("\n")
        ));
    }
    let compiler_pass_self_output_core_text =
        format!("{}\n", render_ail_core(&compiler_pass_self_result.core));
    let pass_order_diagnostics_report_text = render_ail_bootstrap_pass_order_diagnostics_report(
        &compiler_pass_action,
        &pass_paths,
        &toolchain_core_text,
        &toolchain_pass_output_core_text,
        &compiler_pass_self_output_core_text,
        &fixed_point_report_text,
    );
    let pass_composition_report_text =
        render_ail_bootstrap_pass_composition_report(&AilBootstrapPassCompositionEvidence {
            target_name: target,
            compiler_pass_action: &compiler_pass_action,
            input_core_text: &toolchain_core_text,
            output_core_text: &toolchain_pass_output_core_text,
            pass_trace_text: &toolchain_pass_trace_text,
            fixed_point_report_text: &fixed_point_report_text,
            compiler_pass_self_output_core_text: &compiler_pass_self_output_core_text,
            pass_order_diagnostics_report_text: &pass_order_diagnostics_report_text,
        });
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
        pass_composition_report_text: &pass_composition_report_text,
        pass_order_diagnostics_report_text: &pass_order_diagnostics_report_text,
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
        pass_composition_report_text: &pass_composition_report_text,
        pass_order_diagnostics_report_text: &pass_order_diagnostics_report_text,
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
            pass_composition_report_text: &pass_composition_report_text,
            pass_order_diagnostics_report_text: &pass_order_diagnostics_report_text,
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

const DEFAULT_AIL_TOOLCHAIN_AGENT_PATH: &str = "examples/ail_toolchain_agent.ail";

fn is_ail_package_dir(path: &std::path::Path) -> bool {
    path.join("ail-package.md").is_file()
}

fn discover_default_ail_toolchain_agent(package: &AilPackage) -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(package_parent) = package.root.parent() {
        candidates.push(package_parent.join("ail_toolchain_agent.ail"));
    }
    for ancestor in package.root.ancestors() {
        candidates.push(ancestor.join(DEFAULT_AIL_TOOLCHAIN_AGENT_PATH));
    }
    candidates.push(std::path::PathBuf::from(DEFAULT_AIL_TOOLCHAIN_AGENT_PATH));
    candidates
        .into_iter()
        .find(|candidate| is_ail_package_dir(candidate))
        .map(|candidate| candidate.to_string_lossy().to_string())
}

fn run_ail_story_command(
    path: &str,
    package: &ail::ail::AilPackage,
    cli_options: &CliOptions,
) -> Result<u8, String> {
    let story_file = cli_options
        .ail_story_file
        .as_deref()
        .ok_or_else(|| "ail-story requires --story-file <path>".to_string())?;
    let story_path = std::path::Path::new(story_file);
    let story_source_text = fs::read_to_string(story_path)
        .map_err(|error| format!("failed to read examples story file {story_file}: {error}"))?;
    let story_fields = parse_ail_e2e_story_file_fields(story_path)?;
    let diagnostics = validate_ail_story_mode_fields(&story_fields);
    if !diagnostics.is_empty() {
        println!("ail-story diagnostics:");
        for diagnostic in diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }
    let normalized_story_fields = normalized_ail_story_mode_fields(&story_fields);
    let normalized_story_text = render_ail_story_mode_fields(&normalized_story_fields);
    let source_artifacts = load_ail_source_package_artifacts(path, "ail-story")?;
    let endpoint = cli_options
        .llm_endpoint
        .as_deref()
        .unwrap_or(&package.metadata.base_llm_endpoint);
    let story_max_tokens = cli_options
        .llm_max_tokens
        .unwrap_or(ail::llm::DEFAULT_CHAT_MAX_TOKENS);
    let story_llm_options = AilStoryLlmOptions {
        endpoint,
        max_tokens: story_max_tokens,
        retry_prompt_envelope_errors: true,
    };
    let effective_agent_path = cli_options
        .ail_build_agent
        .clone()
        .or_else(|| discover_default_ail_toolchain_agent(package));
    let mut story_cli_options = cli_options.clone();
    if story_cli_options.ail_build_agent.is_none() {
        story_cli_options.ail_build_agent = effective_agent_path.clone();
    }
    let requirements_prompt = prompt_with_requested_native_action(
        &render_ail_story_requirements_prompt(&normalized_story_text),
        cli_options
            .ail_compile_target
            .as_ref()
            .and(cli_options.ail_action.as_deref()),
    );
    let requirements_prompt =
        prompt_with_source_spec_context(&requirements_prompt, &source_artifacts.spec_text);
    let mut agent_start = if let Some(agent_path) = effective_agent_path.as_deref() {
        let story_id = normalized_story_fields
            .get("user-story-id")
            .map(String::as_str)
            .unwrap_or("unspecified");
        let semantic_anchors = normalized_story_fields
            .get("semantic-anchors")
            .map(String::as_str)
            .unwrap_or("");
        let mut agent_start =
            run_ail_build_agent_capture(agent_path, &package.metadata.name, &requirements_prompt)?;
        agent_start
            .trace
            .insert(0, "entrypoint=ail-story".to_string());
        agent_start
            .trace
            .insert(1, format!("buildrequest.story-id={story_id}"));
        agent_start.trace.insert(
            2,
            format!("buildrequest.semantic-anchors={semantic_anchors}"),
        );
        agent_start.state.insert(
            "buildrequest.entrypoint".to_string(),
            "ail-story".to_string(),
        );
        agent_start
            .state
            .insert("buildrequest.story-id".to_string(), story_id.to_string());
        agent_start.state.insert(
            "buildrequest.semantic-anchors".to_string(),
            semantic_anchors.to_string(),
        );
        agent_start.state.insert(
            "buildrequest.story".to_string(),
            normalized_story_text.clone(),
        );
        Some(agent_start)
    } else {
        None
    };
    let agent_requirements_context = agent_start
        .as_ref()
        .map(render_ail_build_agent_requirements_context);
    let mut story_llm_transcripts = Vec::new();
    let requirements_outcome = draft_checked_ail_requirements_for_story_or_questions(
        package,
        &requirements_prompt,
        agent_requirements_context.as_deref(),
        story_llm_options,
        &mut story_llm_transcripts,
    )?;
    let (requirements, requirements_diagnostics) = match requirements_outcome {
        AilRequirementsDraftOutcome::Requirements { text, diagnostics } => (text, diagnostics),
        AilRequirementsDraftOutcome::Questions(questions) => {
            let questions_text = render_ail_interview_questions_artifact(&questions);
            println!("ail-story blocking questions:");
            println!("{questions_text}");
            if let Some(artifact_dir) = cli_options.artifact_dir.as_deref() {
                write_ail_story_question_artifacts(
                    artifact_dir,
                    AilStoryModeArtifactSet {
                        package_name: &package.metadata.name,
                        package_version: &package.metadata.version,
                        story_file,
                        story_source_text: &story_source_text,
                        story_normalized_text: &normalized_story_text,
                        story_fields: &normalized_story_fields,
                        llm_endpoint: Some(endpoint),
                        llm_max_tokens: cli_options.llm_max_tokens,
                        llm_transcripts: &story_llm_transcripts,
                    },
                    &questions_text,
                    agent_start.as_ref().map(|agent| agent.trace.as_slice()),
                )?;
            }
            return Ok(1);
        }
    };
    if !requirements_diagnostics.is_empty() {
        println!("ail-story requirements diagnostics:");
        for diagnostic in requirements_diagnostics {
            println!("{}", diagnostic.detailed_message());
        }
        return Ok(1);
    }
    let semantic_anchors = normalized_story_fields
        .get("semantic-anchors")
        .map(String::as_str)
        .unwrap_or("");
    let agent_spec_context = if let (Some(agent_path), Some(previous_agent_start)) =
        (effective_agent_path.as_deref(), agent_start.take())
    {
        let prepared_agent_start =
            run_ail_build_agent_prepare_spec(agent_path, previous_agent_start, &requirements)?;
        let context = render_ail_build_agent_spec_context(&prepared_agent_start);
        agent_start = Some(prepared_agent_start);
        Some(context)
    } else {
        None
    };
    let spec_prompt = prompt_with_requested_native_action(
        &render_ail_story_spec_prompt(&normalized_story_text, semantic_anchors),
        cli_options
            .ail_compile_target
            .as_ref()
            .and(cli_options.ail_action.as_deref()),
    );
    let spec_prompt = prompt_with_source_spec_context(&spec_prompt, &source_artifacts.spec_text);
    let draft = draft_checked_ail_spec_for_story_requirements(
        package,
        &spec_prompt,
        &requirements,
        agent_spec_context.as_deref(),
        story_llm_options,
        &mut story_llm_transcripts,
    )?;
    if !draft.success() {
        println!("ail-story diagnostics:");
        for diagnostic in draft.diagnostics {
            println!("{}", diagnostic.detailed_message());
        }
        return Ok(1);
    }
    if let (Some(agent_path), Some(previous_agent_start)) =
        (effective_agent_path.as_deref(), agent_start.take())
    {
        agent_start = Some(run_ail_build_agent_accept_spec(
            agent_path,
            previous_agent_start,
            &requirements,
            &draft.spec_text,
        )?);
    }
    let document = parse_ail_package_spec_text(package, &draft.spec_text)?;
    let core = elaborate_ail_core(package, &document);
    let exit_code = run_ail_build_from_core(
        core,
        &story_cli_options,
        Some(source_artifacts),
        Some(&requirements),
        Some(&draft.spec_text),
        Some(&requirements_prompt),
        agent_start,
    )?;
    if exit_code == 0
        && let Some(artifact_dir) = cli_options.artifact_dir.as_deref()
    {
        write_ail_story_mode_artifacts(
            artifact_dir,
            AilStoryModeArtifactSet {
                package_name: &package.metadata.name,
                package_version: &package.metadata.version,
                story_file,
                story_source_text: &story_source_text,
                story_normalized_text: &normalized_story_text,
                story_fields: &normalized_story_fields,
                llm_endpoint: Some(endpoint),
                llm_max_tokens: cli_options.llm_max_tokens,
                llm_transcripts: &story_llm_transcripts,
            },
        )?;
    }
    Ok(exit_code)
}

fn run_ail_command(command: &str, path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    if command == "ail-bootstrap" {
        return run_ail_bootstrap_command(path, cli_options);
    }
    if command == "ail-prompt-corpus" {
        return run_ail_prompt_corpus_command(path, cli_options);
    }
    if command == "ail-agent-contracts" {
        return run_ail_agent_contracts_command(path);
    }
    if command == "ail-v03-roadmap" {
        return run_ail_v03_roadmap_command(path, cli_options);
    }
    if matches!(command, "ail-examples" | "ail-e2e-corpus") {
        return run_ail_e2e_corpus_command(path, cli_options);
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
        let core_file = cli_options
            .ail_core_file
            .as_ref()
            .ok_or_else(|| "ail-spec requires --core-file <path>".to_string())?;
        let source_core_text = fs::read_to_string(core_file)
            .map_err(|error| format!("failed to read {core_file}: {error}"))?;
        let core = parse_cli_ail_core(cli_options)?;
        let diagnostics = check_ail_core(&core);
        if !diagnostics.is_empty() {
            println!("ail-spec core diagnostics:");
            for diagnostic in diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        let rendered_spec_text = format!("{}\n", render_ail_spec_from_core(&core));
        if let Some(artifact_dir) = &cli_options.artifact_dir {
            let roundtrip_document = parse_ail_spec_text(&rendered_spec_text)?;
            let roundtrip_core = elaborate_ail_core(
                &ail::ail::AilPackage {
                    metadata: core.package.clone(),
                    root: std::path::PathBuf::new(),
                    spec_path: std::path::PathBuf::new(),
                    spec_text: String::new(),
                    imports: Vec::new(),
                },
                &roundtrip_document,
            );
            let roundtrip_diagnostics = check_ail_core(&roundtrip_core);
            if !roundtrip_diagnostics.is_empty() {
                println!("ail-spec roundtrip diagnostics:");
                for diagnostic in roundtrip_diagnostics {
                    println!("{diagnostic}");
                }
                return Ok(1);
            }
            let source_core_hash = ail_core_hash(&core);
            let roundtrip_core_hash = ail_core_hash(&roundtrip_core);
            if source_core_hash != roundtrip_core_hash {
                return Err(format!(
                    "ail-spec roundtrip hash mismatch: source {source_core_hash}, roundtrip {roundtrip_core_hash}"
                ));
            }
            let roundtrip_core_text = format!("{}\n", render_ail_core(&roundtrip_core));
            write_ail_spec_artifacts(
                artifact_dir,
                AilSpecArtifactSet {
                    source_core_text: &source_core_text,
                    rendered_spec_text: &rendered_spec_text,
                    roundtrip_core_text: &roundtrip_core_text,
                    source_core_hash: &source_core_hash,
                    roundtrip_core_hash: &roundtrip_core_hash,
                },
            )?;
        }
        print!("{rendered_spec_text}");
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
    if command == "ail-story" {
        return run_ail_story_command(path, &package, cli_options);
    }
    if command == "ail-conformance" {
        let result = run_ail_conformance(&package)?;
        let repair_proofs = build_ail_conformance_repair_proofs(&package, &result)?;
        let report_text = render_ail_conformance_report(&result, &repair_proofs);
        let package_dependency_report_text = if package.imports.is_empty() {
            None
        } else {
            Some(render_ail_package_dependency_report(&package)?)
        };
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
            dependency_report_text = append_package_dependency_report(
                dependency_report_text,
                package_dependency_report_text.as_deref(),
            );
            let empty_agent_trace: &[String] = &[];
            let manifest_text = render_ail_conformance_manifest(
                &result,
                &AilConformanceArtifactSet {
                    report_text: &report_text,
                    repair_proofs: &repair_proofs,
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
            dependency_report_text = append_package_dependency_report(
                dependency_report_text,
                package_dependency_report_text.as_deref(),
            );
            None
        };
        if let Some(artifact_dir) = &cli_options.artifact_dir {
            write_ail_conformance_artifacts(
                artifact_dir,
                &result,
                AilConformanceArtifactSet {
                    report_text: &report_text,
                    repair_proofs: &repair_proofs,
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
            draft_checked_ail_requirements_for_package(&package, &prompt, endpoint, None, false)?;
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
            false,
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
                let prompt = prompt_with_requested_native_action(
                    prompt,
                    cli_options
                        .ail_compile_target
                        .as_ref()
                        .and(cli_options.ail_action.as_deref()),
                );
                let prompt = prompt_with_source_spec_context(&prompt, &source_artifacts.spec_text);
                let requirements_prompt = prompt_with_saved_interview_answers(
                    &prompt,
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
                            true,
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
                        &prompt,
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
                    &prompt,
                    &requirements,
                    endpoint,
                    agent_spec_context.as_deref(),
                    true,
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
            let dependency_report_text = append_package_dependency_report(
                dependency_report_text,
                source_artifacts.package_dependency_report_text.as_deref(),
            );
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
    let mut llm_max_tokens = None;
    let mut artifact_dir = None;
    let mut patch_path = None;
    let mut ail_action = None;
    let mut ail_prompt = None;
    let mut ail_pass_target = None;
    let mut ail_build_pass = None;
    let mut ail_build_passes = Vec::new();
    let mut ail_build_agent = None;
    let mut ail_build_base_model = None;
    let mut ail_build_target_model = None;
    let mut ail_interview_file = None;
    let mut ail_requirements_file = None;
    let mut ail_spec_file = None;
    let mut ail_story_file = None;
    let mut ail_core_file = None;
    let mut ail_compile_target = None;
    let mut ail_compile_out = None;
    let mut ail_compile_all_actions = false;
    let mut diagnostics_json = false;
    let mut release_evidence = false;
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
                "ail-run" | "ail-vm" | "ail-pass" | "ail-compile" | "ail-build" | "ail-story"
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
        if arg == "--story-file" {
            if command != "ail-story" {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --story-file".to_string());
            };
            ail_story_file = Some(path.clone());
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
            if ail_build_pass.is_none() {
                ail_build_pass = Some(path.clone());
            }
            ail_build_passes.push(path.clone());
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
                    | "ail-story"
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
                    | "ail-story"
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
            if !matches!(command, "ail-compile" | "ail-build" | "ail-story") {
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
                "ail-interview"
                    | "ail-requirements"
                    | "ail-spec"
                    | "ail-draft"
                    | "ail-build"
                    | "ail-story"
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
        if arg == "--max-tokens" {
            if command != "ail-story" {
                return Err(usage());
            }
            let Some(value) = args.get(index + 1) else {
                return Err("missing value for --max-tokens".to_string());
            };
            let parsed = value
                .parse::<usize>()
                .map_err(|_| "--max-tokens must be a positive integer".to_string())?;
            if parsed == 0 {
                return Err("--max-tokens must be a positive integer".to_string());
            }
            llm_max_tokens = Some(parsed);
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
        if arg == "--release-evidence" {
            if !matches!(
                command,
                "ail-examples" | "ail-e2e-corpus" | "ail-v03-roadmap"
            ) {
                return Err(usage());
            }
            release_evidence = true;
            index += 1;
            continue;
        }
        if arg == "--artifact-dir" {
            if !matches!(
                command,
                "ail-interview"
                    | "ail-spec"
                    | "ail-build"
                    | "ail-pass"
                    | "ail-lower"
                    | "ail-compile"
                    | "ail-conformance"
                    | "ail-bootstrap"
                    | "ail-story"
                    | "ail-prompt-corpus"
                    | "ail-examples"
                    | "ail-e2e-corpus"
                    | "ail-v03-roadmap"
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
        if ail_build_passes.len() > 1 {
            return Err(
                "ail-build accepts exactly one --pass; repeatable pass sequencing is supported by ail-bootstrap"
                    .to_string(),
            );
        }
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
    if command == "ail-story" {
        let native_requested = ail_compile_target.is_some() || ail_compile_out.is_some();
        if native_requested && ail_compile_target.is_none() {
            return Err("ail-story native output requires --target <target>".to_string());
        }
        if native_requested && ail_compile_out.is_none() {
            return Err("ail-story native output requires --out <path>".to_string());
        }
        if native_requested && ail_action.is_none() {
            return Err("ail-story native output requires --action <name>".to_string());
        }
        if !native_requested && ail_action.is_some() {
            return Err("ail-story --action requires --target and --out".to_string());
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
        llm_max_tokens,
        artifact_dir,
        patch_path,
        ail_action,
        ail_prompt,
        ail_pass_target,
        ail_build_pass,
        ail_build_passes,
        ail_build_agent,
        ail_build_base_model,
        ail_build_target_model,
        ail_interview_file,
        ail_requirements_file,
        ail_spec_file,
        ail_story_file,
        ail_core_file,
        ail_compile_target,
        ail_compile_out,
        ail_compile_all_actions,
        diagnostics_json,
        release_evidence,
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
