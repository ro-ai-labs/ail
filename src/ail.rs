use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core_model::{Edge, Graph, Node, attr, json_string};

pub const DEFAULT_BASE_LLM_ENDPOINT: &str = "http://inteligentia-pro-1:8080/v1/chat/completions";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilPackage {
    pub metadata: AilPackageMetadata,
    pub root: PathBuf,
    pub spec_path: PathBuf,
    pub spec_text: String,
    pub imports: Vec<AilLoadedImport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilPackageMetadata {
    pub name: String,
    pub version: String,
    pub profile: String,
    pub entry: String,
    pub features: Vec<String>,
    pub imports: Vec<AilImportSpec>,
    pub conformance: String,
    pub base_llm_endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilImportSpec {
    pub path: String,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilLoadedImport {
    pub spec: AilImportSpec,
    pub package: Box<AilPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilDocument {
    pub application: AilApplication,
    pub things: BTreeMap<String, AilThing>,
    pub tools: BTreeMap<String, AilTool>,
    pub compiler_passes: BTreeMap<String, AilCompilerPass>,
    pub system_components: BTreeMap<String, AilSystemComponent>,
    pub actions: BTreeMap<String, AilAction>,
    pub failures: BTreeMap<String, AilFailure>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilApplication {
    pub name: String,
    pub purpose: String,
    pub users: Vec<String>,
    pub views: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilThing {
    pub name: String,
    pub fields: BTreeMap<String, AilField>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilField {
    pub name: String,
    pub type_name: String,
    pub is_secret: bool,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilTool {
    pub name: String,
    pub label: String,
    pub requirements: Vec<String>,
    pub inputs: BTreeMap<String, AilToolSlot>,
    pub outputs: BTreeMap<String, AilToolSlot>,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
    pub calls: Vec<String>,
    pub permissions: Vec<String>,
    pub approvals: Vec<String>,
    pub failures: Vec<String>,
    pub guarantees: Vec<String>,
    pub traces: Vec<String>,
    pub secret_protections: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilToolSlot {
    pub name: String,
    pub type_name: String,
    pub is_secret: bool,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilCompilerPass {
    pub name: String,
    pub label: String,
    pub purpose: String,
    pub inputs: BTreeMap<String, AilPassValue>,
    pub outputs: BTreeMap<String, AilPassValue>,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
    pub steps: Vec<String>,
    pub failures: Vec<String>,
    pub guarantees: Vec<String>,
    pub traces: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilPassValue {
    pub name: String,
    pub type_name: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilSystemComponent {
    pub name: String,
    pub label: String,
    pub resources: BTreeMap<String, AilSystemResource>,
    pub owned_resources: Vec<String>,
    pub borrowed_resources: Vec<String>,
    pub mutably_borrowed_resources: Vec<String>,
    pub resource_regions: Vec<AilSystemResourceRegion>,
    pub resource_layouts: Vec<AilSystemResourceLayout>,
    pub resource_allocations: Vec<AilSystemResourceAllocation>,
    pub lock_guards: Vec<AilSystemLockGuard>,
    pub execution_contexts: Vec<AilSystemExecutionContext>,
    pub interrupt_priorities: Vec<AilSystemInterruptPriority>,
    pub interrupt_masks: Vec<AilSystemInterruptMask>,
    pub scheduler_tasks: Vec<AilSystemSchedulerTask>,
    pub scheduler_task_priorities: Vec<AilSystemSchedulerTaskPriority>,
    pub scheduler_task_timings: Vec<AilSystemSchedulerTaskTiming>,
    pub capabilities: Vec<String>,
    pub effects: Vec<String>,
    pub guarantees: Vec<String>,
    pub traces: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemResource {
    pub name: String,
    pub type_name: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemResourceRegion {
    pub resource_name: String,
    pub region_name: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemResourceLayout {
    pub resource_name: String,
    pub layout: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemResourceAllocation {
    pub resource_name: String,
    pub placement: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemLockGuard {
    pub resource_name: String,
    pub lock_name: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemExecutionContext {
    pub name: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemInterruptPriority {
    pub context_name: String,
    pub priority: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemInterruptMask {
    pub context_name: String,
    pub mask: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemSchedulerTask {
    pub task_name: String,
    pub context_name: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemSchedulerTaskPriority {
    pub task_name: String,
    pub priority: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilSystemSchedulerTaskTiming {
    pub task_name: String,
    pub deadline: String,
    pub budget: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilAction {
    pub name: String,
    pub label: String,
    pub trigger: String,
    pub requirements: Vec<String>,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
    pub failures: Vec<String>,
    pub guarantees: Vec<String>,
    pub traces: Vec<String>,
    pub secret_protections: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilFailure {
    pub name: String,
    pub condition: String,
    pub handling: Vec<String>,
    pub traces: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilCore {
    pub package: AilPackageMetadata,
    pub graph: Graph,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilBytecodeProgram {
    pub package_name: String,
    pub package_version: String,
    pub profile: String,
    pub actions: BTreeMap<String, AilBytecodeAction>,
    pub failures: BTreeMap<String, AilBytecodeFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilBytecodeAction {
    pub name: String,
    pub instructions: Vec<AilBytecodeInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilBytecodeInstruction {
    pub opcode: String,
    pub operands: BTreeMap<String, String>,
}

impl AilBytecodeInstruction {
    fn new(opcode: impl Into<String>, operands: &[(&str, String)]) -> Self {
        Self {
            opcode: opcode.into(),
            operands: operands
                .iter()
                .map(|(key, value)| ((*key).to_string(), value.clone()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilBytecodeFailure {
    pub name: String,
    pub traces: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AilJsonValue {
    Object(BTreeMap<String, AilJsonValue>),
    Array(Vec<AilJsonValue>),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilRunResult {
    pub status: String,
    pub failure: Option<String>,
    pub final_state: BTreeMap<String, String>,
    pub trace: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilCompilerPassRunResult {
    pub core: AilCore,
    pub run: AilRunResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilDraftResult {
    pub spec_text: String,
    pub diagnostics: Vec<AilDiagnostic>,
}

impl AilDraftResult {
    pub fn success(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilConformanceResult {
    pub package_name: String,
    pub accepted_fixture: String,
    pub accepted_diagnostics: Vec<AilDiagnostic>,
    pub accepted: Vec<AilAcceptedConformanceResult>,
    pub rejected: Vec<AilRejectedConformanceResult>,
}

impl AilConformanceResult {
    pub fn success(&self) -> bool {
        self.accepted_diagnostics.is_empty()
            && self
                .accepted
                .iter()
                .all(|fixture| fixture.diagnostics.is_empty())
            && self
                .rejected
                .iter()
                .all(|fixture| !fixture.diagnostics.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilAcceptedConformanceResult {
    pub fixture: String,
    pub diagnostics: Vec<AilDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilRejectedConformanceResult {
    pub fixture: String,
    pub diagnostics: Vec<AilDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub source_provenance: Option<String>,
    pub affected_graph_item: Option<String>,
    pub repair_suggestion: Option<String>,
}

impl AilDiagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: "error".to_string(),
            source_provenance: None,
            affected_graph_item: None,
            repair_suggestion: None,
        }
    }

    fn from_message(message: impl Into<String>) -> Self {
        let message = message.into();
        let Some((code, rest)) = message.split_once(' ') else {
            return Self::error("", message);
        };
        if code.starts_with("AIL") {
            Self::error(code, rest)
        } else {
            Self::error("", message)
        }
    }

    fn with_source_provenance(mut self, source_provenance: Option<String>) -> Self {
        self.source_provenance = source_provenance;
        self
    }

    fn with_affected_graph_item(mut self, affected_graph_item: impl Into<String>) -> Self {
        self.affected_graph_item = Some(affected_graph_item.into());
        self
    }

    fn with_repair_suggestion(mut self, repair_suggestion: impl Into<String>) -> Self {
        self.repair_suggestion = Some(repair_suggestion.into());
        self
    }

    pub fn detailed_message(&self) -> String {
        let mut details = Vec::new();
        if let Some(source) = &self.source_provenance {
            details.push(format!("source={source}"));
        }
        if let Some(item) = &self.affected_graph_item {
            details.push(format!("graph={item}"));
        }
        if let Some(repair) = &self.repair_suggestion {
            details.push(format!("repair={repair}"));
        }
        if details.is_empty() {
            self.to_string()
        } else {
            format!("{} [{}]", self, details.join("; "))
        }
    }
}

impl Display for AilDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.code.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{} {}", self.code, self.message)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilPatch {
    pub target: AilPatchTarget,
    pub changes: Vec<AilPatchChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AilPatchTarget {
    Application(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AilPatchChange {
    AddField {
        thing_name: String,
        field_name: String,
        type_name: String,
    },
    AddView(String),
    AddAction(AilAction),
}

pub fn load_ail_package_dir(path: impl AsRef<Path>) -> Result<AilPackage, String> {
    let mut stack = BTreeSet::new();
    load_ail_package_dir_inner(path.as_ref(), &mut stack)
}

fn load_ail_package_dir_inner(
    path: &Path,
    stack: &mut BTreeSet<PathBuf>,
) -> Result<AilPackage, String> {
    let root = path.to_path_buf();
    let canonical_root = fs::canonicalize(&root)
        .map_err(|error| format!("failed to resolve {}: {error}", root.display()))?;
    if !stack.insert(canonical_root.clone()) {
        return Err(format!(
            "AIL package import cycle at {}",
            canonical_root.display()
        ));
    }
    let metadata_path = root.join("ail-package.md");
    let metadata_text = fs::read_to_string(&metadata_path)
        .map_err(|error| format!("failed to read {}: {error}", metadata_path.display()))?;
    let metadata = parse_package_metadata(&metadata_text)?;
    let spec_path = root.join(&metadata.entry);
    let spec_text = fs::read_to_string(&spec_path)
        .map_err(|error| format!("failed to read {}: {error}", spec_path.display()))?;
    let mut imports = Vec::new();
    for import in &metadata.imports {
        let import_root = root.join(&import.path);
        let package = load_ail_package_dir_inner(&import_root, stack)?;
        imports.push(AilLoadedImport {
            spec: import.clone(),
            package: Box::new(package),
        });
    }
    stack.remove(&canonical_root);
    Ok(AilPackage {
        metadata,
        root,
        spec_path,
        spec_text,
        imports,
    })
}

pub fn parse_ail_package_document(package: &AilPackage) -> Result<AilDocument, String> {
    parse_ail_package_spec_text(package, &package.spec_text)
}

pub fn parse_ail_package_spec_text(
    package: &AilPackage,
    text: &str,
) -> Result<AilDocument, String> {
    let mut document = parse_ail_spec_text(text)?;
    for import in &package.imports {
        let imported = parse_ail_package_document(&import.package)?;
        merge_ail_import(
            &mut document,
            namespace_ail_document(&imported, &import.spec.alias),
        );
    }
    Ok(document)
}

pub fn parse_ail_patch_text(text: &str) -> Result<AilPatch, String> {
    let mut target = None;
    let mut changes = Vec::new();
    let mut section: Option<&str> = None;
    let mut current_action: Option<AilAction> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("patch ") {
            continue;
        }
        if line == "target:" {
            if let Some(action) = current_action.take() {
                changes.push(AilPatchChange::AddAction(action));
            }
            section = Some("target");
            continue;
        }
        if line == "change:" {
            if let Some(action) = current_action.take() {
                changes.push(AilPatchChange::AddAction(action));
            }
            section = Some("change");
            continue;
        }

        match section {
            Some("target") => {
                if let Some(app_name) = line.strip_prefix("app ") {
                    target = Some(AilPatchTarget::Application(app_name.trim().to_string()));
                }
            }
            Some("change") => {
                if let Some(field) = parse_ail_patch_field(line) {
                    if let Some(action) = current_action.take() {
                        changes.push(AilPatchChange::AddAction(action));
                    }
                    changes.push(field);
                    continue;
                }
                if let Some(view) = line.strip_prefix("add view ") {
                    if let Some(action) = current_action.take() {
                        changes.push(AilPatchChange::AddAction(action));
                    }
                    changes.push(AilPatchChange::AddView(view.trim().to_string()));
                    continue;
                }
                if let Some(label) = line.strip_prefix("add action ") {
                    if let Some(action) = current_action.take() {
                        changes.push(AilPatchChange::AddAction(action));
                    }
                    let label = label.trim().to_string();
                    let name = action_name_from_label(&label);
                    current_action = Some(AilAction {
                        name: name.clone(),
                        label,
                        provenance: format!("action:{name}"),
                        ..AilAction::default()
                    });
                    continue;
                }
                if let Some(action) = current_action.as_mut() {
                    apply_ail_patch_action_line(action, line);
                }
            }
            _ => {}
        }
    }

    if let Some(action) = current_action.take() {
        changes.push(AilPatchChange::AddAction(action));
    }
    let target = target.ok_or_else(|| "AIL patch must declare target".to_string())?;
    if changes.is_empty() {
        return Err("AIL patch must declare at least one change".to_string());
    }
    Ok(AilPatch { target, changes })
}

pub fn apply_ail_patch(document: &AilDocument, patch: &AilPatch) -> Result<AilDocument, String> {
    let mut document = document.clone();
    let AilPatchTarget::Application(target_app) = &patch.target;
    if document.application.name != *target_app {
        return Err(format!("unknown AIL application target '{target_app}'"));
    }
    for change in &patch.changes {
        match change {
            AilPatchChange::AddField {
                thing_name,
                field_name,
                type_name,
            } => {
                let Some(thing) = document.things.get_mut(thing_name) else {
                    return Err(format!("unknown AIL thing '{thing_name}'"));
                };
                thing.fields.insert(
                    field_name.clone(),
                    AilField {
                        name: field_name.clone(),
                        type_name: normalize_type_name(type_name),
                        is_secret: type_contains_secret(type_name),
                        provenance: format!("field:{thing_name}.{field_name}"),
                    },
                );
            }
            AilPatchChange::AddView(view) => {
                if !document.application.views.contains(view) {
                    document.application.views.push(view.clone());
                }
            }
            AilPatchChange::AddAction(action) => {
                document.actions.insert(action.name.clone(), action.clone());
            }
        }
    }
    Ok(document)
}

pub fn apply_ail_core_patch_text(core: &AilCore, patch_text: &str) -> Result<AilCore, String> {
    let mut parser = AilJsonParser::new(patch_text);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if !parser.is_finished() {
        return Err("unexpected trailing content in AIL-Core patch artifact".to_string());
    }
    let root = value
        .as_object()
        .ok_or_else(|| "AIL-Core patch artifact must be a JSON object".to_string())?;
    let schema = required_json_string_for(root, "schema", "AIL-Core patch")?;
    if schema != "ail-core.patch.v0" {
        return Err(format!("expected ail-core.patch.v0 patch, got '{schema}'"));
    }
    let base_hash = required_json_string_for(root, "base_hash", "AIL-Core patch")?;
    let actual_hash = ail_core_hash(core);
    if base_hash != actual_hash {
        return Err(format!(
            "AIL-Core patch base_hash mismatch: expected {actual_hash}, got {base_hash}"
        ));
    }
    let mut patched = core.clone();
    for op_value in required_json_array_for(root, "ops", "AIL-Core patch")? {
        let op = op_value
            .as_object()
            .ok_or_else(|| "AIL-Core patch op must be an object".to_string())?;
        match required_json_string_for(op, "op", "AIL-Core patch op")? {
            "add_node" => apply_ail_core_patch_add_node(&mut patched, op)?,
            "add_edge" => apply_ail_core_patch_add_edge(&mut patched, op)?,
            "remove_edge" => apply_ail_core_patch_remove_edge(&mut patched, op)?,
            "replace_edge_attributes" => {
                apply_ail_core_patch_replace_edge_attributes(&mut patched, op)?
            }
            "replace_node_attributes" => {
                apply_ail_core_patch_replace_node_attributes(&mut patched, op)?
            }
            op_name => return Err(format!("unsupported AIL-Core patch op '{op_name}'")),
        }
    }
    Ok(patched)
}

pub fn ail_core_hash(core: &AilCore) -> String {
    format!("ail-core:{}", ail_text_fingerprint(&render_ail_core(core)))
}

fn ail_text_fingerprint(text: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
}

fn apply_ail_core_patch_add_node(
    core: &mut AilCore,
    op: &BTreeMap<String, AilJsonValue>,
) -> Result<(), String> {
    let kind = required_json_string_for(op, "kind", "AIL-Core patch add_node")?;
    let name = required_json_string_for(op, "name", "AIL-Core patch add_node")?;
    let type_name = optional_json_string(op, "type").map(ToString::to_string);
    let attributes = optional_json_string_map(op, "attributes", "AIL-Core patch add_node")?;
    let node = core
        .graph
        .add_node(kind.to_string(), name.to_string(), type_name, attributes);
    for provenance in optional_json_string_array(op, "provenance", "AIL-Core patch add_node")? {
        attach_provenance(&mut core.graph, &node, provenance);
    }
    Ok(())
}

fn apply_ail_core_patch_add_edge(
    core: &mut AilCore,
    op: &BTreeMap<String, AilJsonValue>,
) -> Result<(), String> {
    let kind = required_json_string_for(op, "kind", "AIL-Core patch add_edge")?;
    let source_label = required_json_string_for(op, "source", "AIL-Core patch add_edge")?;
    let target_label = required_json_string_for(op, "target", "AIL-Core patch add_edge")?;
    let source = find_core_patch_node(core, source_label).ok_or_else(|| {
        format!("AIL-Core patch add_edge references unknown source '{source_label}'")
    })?;
    let target = find_core_patch_node(core, target_label).ok_or_else(|| {
        format!("AIL-Core patch add_edge references unknown target '{target_label}'")
    })?;
    let mut attributes = optional_json_string_map(op, "attributes", "AIL-Core patch add_edge")?;
    if !attributes.contains_key("provenance")
        && let Some(provenance) =
            optional_json_string_array(op, "provenance", "AIL-Core patch add_edge")?
                .into_iter()
                .next()
    {
        attributes.insert("provenance".to_string(), provenance);
    }
    core.graph
        .add_edge(kind.to_string(), &source, &target, attributes);
    Ok(())
}

fn apply_ail_core_patch_remove_edge(
    core: &mut AilCore,
    op: &BTreeMap<String, AilJsonValue>,
) -> Result<(), String> {
    let kind = required_json_string_for(op, "kind", "AIL-Core patch remove_edge")?;
    let source_label = required_json_string_for(op, "source", "AIL-Core patch remove_edge")?;
    let target_label = required_json_string_for(op, "target", "AIL-Core patch remove_edge")?;
    let source = find_core_patch_node(core, source_label).ok_or_else(|| {
        format!("AIL-Core patch remove_edge references unknown source '{source_label}'")
    })?;
    let target = find_core_patch_node(core, target_label).ok_or_else(|| {
        format!("AIL-Core patch remove_edge references unknown target '{target_label}'")
    })?;
    let original_edge_count = core.graph.edges.len();
    core.graph.edges.retain(|edge| {
        !(edge.kind == kind && edge.source == source.id && edge.target == target.id)
    });
    if core.graph.edges.len() == original_edge_count {
        return Err(format!(
            "AIL-Core patch remove_edge did not find edge {kind} {source_label} -> {target_label}"
        ));
    }
    Ok(())
}

fn apply_ail_core_patch_replace_edge_attributes(
    core: &mut AilCore,
    op: &BTreeMap<String, AilJsonValue>,
) -> Result<(), String> {
    let kind = required_json_string_for(op, "kind", "AIL-Core patch replace_edge_attributes")?;
    let source_label =
        required_json_string_for(op, "source", "AIL-Core patch replace_edge_attributes")?;
    let target_label =
        required_json_string_for(op, "target", "AIL-Core patch replace_edge_attributes")?;
    let replacement_attributes =
        optional_json_string_map(op, "attributes", "AIL-Core patch replace_edge_attributes")?;
    if replacement_attributes.is_empty() {
        return Err("AIL-Core patch replace_edge_attributes must provide attributes".to_string());
    }
    let source = find_core_patch_node(core, source_label).ok_or_else(|| {
        format!("AIL-Core patch replace_edge_attributes references unknown source '{source_label}'")
    })?;
    let target = find_core_patch_node(core, target_label).ok_or_else(|| {
        format!("AIL-Core patch replace_edge_attributes references unknown target '{target_label}'")
    })?;
    let edge_index = core
        .graph
        .edges
        .iter()
        .position(|edge| edge.kind == kind && edge.source == source.id && edge.target == target.id)
        .ok_or_else(|| {
            format!(
                "AIL-Core patch replace_edge_attributes did not find edge {kind} {source_label} -> {target_label}"
            )
        })?;
    let mut attributes = core.graph.edges[edge_index].attributes.clone();
    for (key, value) in replacement_attributes {
        attributes.insert(key, value);
    }
    core.graph.edges[edge_index] = Edge::new(kind.to_string(), &source, &target, attributes);
    Ok(())
}

fn apply_ail_core_patch_replace_node_attributes(
    core: &mut AilCore,
    op: &BTreeMap<String, AilJsonValue>,
) -> Result<(), String> {
    let target_label =
        required_json_string_for(op, "target", "AIL-Core patch replace_node_attributes")?;
    let target = find_core_patch_node(core, target_label).ok_or_else(|| {
        format!("AIL-Core patch replace_node_attributes references unknown target '{target_label}'")
    })?;
    let replacement_attributes =
        optional_json_string_map(op, "attributes", "AIL-Core patch replace_node_attributes")?;
    let replacement_type = optional_json_string(op, "type").map(ToString::to_string);
    if replacement_attributes.is_empty() && replacement_type.is_none() {
        return Err(
            "AIL-Core patch replace_node_attributes must provide attributes or type".to_string(),
        );
    }
    let node_index = core
        .graph
        .nodes
        .iter()
        .position(|node| node.id == target.id)
        .ok_or_else(|| {
            format!("AIL-Core patch replace_node_attributes lost target '{target_label}'")
        })?;
    let original = core.graph.nodes[node_index].clone();
    let mut attributes = original.attributes.clone();
    for (key, value) in replacement_attributes {
        attributes.insert(key, value);
    }
    let updated = Node::new(
        original.kind.clone(),
        original.name.clone(),
        replacement_type.or(original.type_name.clone()),
        attributes,
    );
    core.graph.nodes[node_index] = updated.clone();
    rewire_core_graph_node_id(&mut core.graph, &original.id, &updated.id)?;
    for provenance in
        optional_json_string_array(op, "provenance", "AIL-Core patch replace_node_attributes")?
    {
        attach_provenance(&mut core.graph, &updated, provenance);
    }
    Ok(())
}

fn rewire_core_graph_node_id(graph: &mut Graph, old_id: &str, new_id: &str) -> Result<(), String> {
    if old_id == new_id {
        return Ok(());
    }
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.clone()))
        .collect::<BTreeMap<_, _>>();
    for edge in &mut graph.edges {
        let rewired = edge.source == old_id || edge.target == old_id;
        if edge.source == old_id {
            edge.source = new_id.to_string();
        }
        if edge.target == old_id {
            edge.target = new_id.to_string();
        }
        if rewired {
            let source = node_by_id.get(&edge.source).ok_or_else(|| {
                format!(
                    "AIL-Core patch produced missing edge source {}",
                    edge.source
                )
            })?;
            let target = node_by_id.get(&edge.target).ok_or_else(|| {
                format!(
                    "AIL-Core patch produced missing edge target {}",
                    edge.target
                )
            })?;
            *edge = Edge::new(edge.kind.clone(), source, target, edge.attributes.clone());
        }
    }
    Ok(())
}

fn find_core_patch_node(core: &AilCore, label: &str) -> Option<Node> {
    core.graph
        .nodes
        .iter()
        .find(|node| node.id == label || core_node_label(node) == label)
        .cloned()
}

fn required_json_string_for<'a>(
    object: &'a BTreeMap<String, AilJsonValue>,
    key: &str,
    context: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(AilJsonValue::as_string)
        .ok_or_else(|| format!("{context} field '{key}' must be a string"))
}

fn required_json_array_for<'a>(
    object: &'a BTreeMap<String, AilJsonValue>,
    key: &str,
    context: &str,
) -> Result<&'a [AilJsonValue], String> {
    object
        .get(key)
        .and_then(AilJsonValue::as_array)
        .ok_or_else(|| format!("{context} field '{key}' must be an array"))
}

fn optional_json_string<'a>(
    object: &'a BTreeMap<String, AilJsonValue>,
    key: &str,
) -> Option<&'a str> {
    object.get(key).and_then(AilJsonValue::as_string)
}

fn optional_json_string_array(
    object: &BTreeMap<String, AilJsonValue>,
    key: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    let Some(value) = object.get(key) else {
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| format!("{context} field '{key}' must be an array"))?;
    array
        .iter()
        .map(|value| {
            value
                .as_string()
                .map(ToString::to_string)
                .ok_or_else(|| format!("{context} field '{key}' entries must be strings"))
        })
        .collect()
}

fn optional_json_string_map(
    object: &BTreeMap<String, AilJsonValue>,
    key: &str,
    context: &str,
) -> Result<BTreeMap<String, String>, String> {
    let Some(value) = object.get(key) else {
        return Ok(BTreeMap::new());
    };
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context} field '{key}' must be an object"))?;
    object
        .iter()
        .map(|(key, value)| {
            value
                .as_string()
                .map(|value| (key.clone(), value.to_string()))
                .ok_or_else(|| format!("{context} field '{key}' values must be strings"))
        })
        .collect()
}

pub fn parse_ail_spec_text(text: &str) -> Result<AilDocument, String> {
    let mut document = AilDocument {
        application: AilApplication::default(),
        things: BTreeMap::new(),
        tools: BTreeMap::new(),
        compiler_passes: BTreeMap::new(),
        system_components: BTreeMap::new(),
        actions: BTreeMap::new(),
        failures: BTreeMap::new(),
    };
    let mut current_thing: Option<String> = None;
    let mut current_tool: Option<String> = None;
    let mut current_tool_section: Option<ToolSection> = None;
    let mut current_compiler_pass: Option<String> = None;
    let mut current_compiler_pass_section: Option<CompilerPassSection> = None;
    let mut current_system_component: Option<String> = None;
    let mut current_system_section: Option<SystemSection> = None;
    let mut current_action: Option<String> = None;
    let mut current_failure: Option<String> = None;
    let mut current_list: Option<ListContext> = None;
    let mut continuation: Option<ContinuationTarget> = None;
    let mut action_header_waiting_for_when = false;

    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(target) = continuation.take()
            && !line.starts_with("- ")
            && !is_structural_line(line)
        {
            append_continuation(&mut document, &target, line);
            if !line.ends_with('.') && !line.ends_with(':') {
                continuation = Some(target);
            }
            continue;
        }
        if let Some((name, purpose)) = parse_application_line(line) {
            document.application.name = name;
            document.application.purpose = purpose;
            if !line.ends_with('.') {
                continuation = Some(ContinuationTarget::ApplicationPurpose);
            }
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            continue;
        }
        if line == "The application has these users:" {
            current_list = Some(ListContext::Users);
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_action = None;
            current_failure = None;
            continue;
        }
        if line == "The application shows:" {
            current_list = Some(ListContext::Views);
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_action = None;
            current_failure = None;
            continue;
        }
        if let Some(thing_name) = parse_thing_header(line) {
            let provenance = format!("thing:{thing_name}");
            document
                .things
                .entry(thing_name.clone())
                .or_insert_with(|| AilThing {
                    name: thing_name.clone(),
                    fields: BTreeMap::new(),
                    provenance,
                });
            current_thing = Some(thing_name);
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            continue;
        }
        if let Some(label) = parse_tool_header(line) {
            let name = action_name_from_label(&label);
            document
                .tools
                .entry(name.clone())
                .or_insert_with(|| AilTool {
                    name: name.clone(),
                    label,
                    provenance: format!("tool:{name}"),
                    ..AilTool::default()
                });
            current_tool = Some(name);
            current_tool_section = None;
            current_system_component = None;
            current_system_section = None;
            current_thing = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(section) = parse_tool_section(line)
            && current_tool.is_some()
        {
            current_tool_section = Some(section);
            current_system_component = None;
            current_system_section = None;
            current_thing = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(label) = parse_compiler_pass_header(line) {
            let name = action_name_from_label(&label);
            document
                .compiler_passes
                .entry(name.clone())
                .or_insert_with(|| AilCompilerPass {
                    name: name.clone(),
                    label,
                    provenance: format!("compiler_pass:{name}"),
                    ..AilCompilerPass::default()
                });
            current_compiler_pass = Some(name);
            current_compiler_pass_section = None;
            current_tool = None;
            current_tool_section = None;
            current_system_component = None;
            current_system_section = None;
            current_thing = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(pass_name) = current_compiler_pass.clone()
            && current_compiler_pass_section.is_none()
            && let Some(purpose) = parse_compiler_pass_purpose_line(line)
        {
            if let Some(pass) = document.compiler_passes.get_mut(&pass_name) {
                append_words(&mut pass.purpose, &purpose);
            }
            if !line.ends_with('.') {
                continuation = Some(ContinuationTarget::CompilerPassPurpose(pass_name));
            }
            continue;
        }
        if let Some(section) = parse_compiler_pass_section(line)
            && current_compiler_pass.is_some()
        {
            current_compiler_pass_section = Some(section);
            current_tool = None;
            current_tool_section = None;
            current_system_component = None;
            current_system_section = None;
            current_thing = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(label) = parse_system_component_header(line) {
            let name = action_name_from_label(&label);
            document
                .system_components
                .entry(name.clone())
                .or_insert_with(|| AilSystemComponent {
                    name: name.clone(),
                    label,
                    provenance: format!("system_component:{name}"),
                    ..AilSystemComponent::default()
                });
            current_system_component = Some(name);
            current_system_section = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_thing = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(section) = parse_system_section(line)
            && current_system_component.is_some()
        {
            current_system_section = Some(section);
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_thing = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(label) = parse_action_header(line) {
            let name = action_name_from_label(&label);
            document
                .actions
                .entry(name.clone())
                .or_insert_with(|| AilAction {
                    name: name.clone(),
                    label,
                    provenance: format!("action:{name}"),
                    ..AilAction::default()
                });
            current_action = Some(name);
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = true;
            continue;
        }
        if let Some(trigger) = parse_when_line(line) {
            let action_name = if action_header_waiting_for_when {
                current_action
                    .clone()
                    .ok_or_else(|| format!("line {line_number}: missing action before trigger"))?
            } else {
                let inferred = infer_action_name_from_trigger(&trigger);
                document
                    .actions
                    .entry(inferred.clone())
                    .or_insert_with(|| AilAction {
                        name: inferred.clone(),
                        label: title_from_pascal_case(&inferred),
                        provenance: format!("action:{inferred}"),
                        ..AilAction::default()
                    });
                inferred
            };
            if let Some(action) = document.actions.get_mut(&action_name) {
                action.trigger = trigger;
            }
            current_action = Some(action_name);
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some((failure_name, condition)) = parse_failure_header(line) {
            if let Some(tool_name) = &current_tool
                && let Some(tool) = document.tools.get_mut(tool_name)
            {
                tool.failures.push(failure_name.clone());
            }
            if let Some(pass_name) = &current_compiler_pass
                && let Some(pass) = document.compiler_passes.get_mut(pass_name)
            {
                pass.failures.push(failure_name.clone());
            }
            let failure = AilFailure {
                name: failure_name.clone(),
                condition,
                provenance: format!("failure:{failure_name}"),
                ..AilFailure::default()
            };
            document.failures.insert(failure_name.clone(), failure);
            if !line.ends_with(':') {
                continuation = Some(ContinuationTarget::FailureCondition(failure_name.clone()));
            }
            current_failure = Some(failure_name);
            current_action = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(bullet) = line.strip_prefix("- ") {
            if let Some(thing_name) = &current_thing {
                parse_field_bullet(&mut document, thing_name, bullet, line_number)?;
                continue;
            }
            if let (Some(tool_name), Some(section)) = (&current_tool, current_tool_section) {
                parse_tool_bullet(&mut document, tool_name, section, bullet, line_number)?;
                continue;
            }
            if let (Some(pass_name), Some(section)) =
                (&current_compiler_pass, current_compiler_pass_section)
            {
                parse_compiler_pass_bullet(&mut document, pass_name, section, bullet, line_number)?;
                continue;
            }
            if let (Some(component_name), Some(section)) =
                (&current_system_component, current_system_section)
            {
                parse_system_bullet(&mut document, component_name, section, bullet, line_number)?;
                continue;
            }
            if let Some(action_name) = &current_action {
                parse_action_bullet(&mut document, action_name, bullet);
                continue;
            }
            if let Some(failure_name) = &current_failure {
                parse_failure_bullet(&mut document, failure_name, bullet);
                continue;
            }
            match current_list {
                Some(ListContext::Users) => document.application.users.push(bullet.to_string()),
                Some(ListContext::Views) => document.application.views.push(wrapped_bullet(bullet)),
                None => {}
            }
        } else if let Some(ListContext::Views) = current_list
            && !document.application.views.is_empty()
        {
            let last = document.application.views.len() - 1;
            document.application.views[last].push(' ');
            document.application.views[last].push_str(line);
        }
    }

    if document.application.name.is_empty()
        && document.tools.is_empty()
        && document.compiler_passes.is_empty()
        && document.system_components.is_empty()
    {
        return Err(
            "AIL-Spec missing application, tool, compiler pass, or system component declaration"
                .to_string(),
        );
    }
    Ok(document)
}

pub fn elaborate_ail_core(package: &AilPackage, document: &AilDocument) -> AilCore {
    let mut graph = Graph::default();
    let application = if document.application.name.is_empty() {
        None
    } else {
        let application = graph.add_node(
            "Application",
            &document.application.name,
            None,
            attr(&[("purpose", &document.application.purpose)]),
        );
        attach_provenance(&mut graph, &application, "application");
        Some(application)
    };

    for user in &document.application.users {
        let user_node = graph.add_node("User", user, None, BTreeMap::new());
        if let Some(application) = &application {
            graph.add_edge("contains", application, &user_node, BTreeMap::new());
        }
        attach_provenance(&mut graph, &user_node, format!("application.user:{user}"));
    }

    for thing in document.things.values() {
        let thing_node = graph.add_node("Thing", &thing.name, None, BTreeMap::new());
        if let Some(application) = &application {
            graph.add_edge("contains", application, &thing_node, BTreeMap::new());
        }
        attach_provenance(&mut graph, &thing_node, &thing.provenance);
        for field in thing.fields.values() {
            let field_node = graph.add_node(
                "Field",
                format!("{}.{}", thing.name, field.name),
                Some(field.type_name.clone()),
                attr(&[("secret", if field.is_secret { "true" } else { "false" })]),
            );
            graph.add_edge("has_field", &thing_node, &field_node, BTreeMap::new());
            attach_provenance(&mut graph, &field_node, &field.provenance);
            if field.is_secret {
                let secret_node =
                    graph.add_node("Secret", field_node.name.clone(), None, BTreeMap::new());
                graph.add_edge(
                    "protects_secret",
                    &secret_node,
                    &field_node,
                    BTreeMap::new(),
                );
                attach_provenance(&mut graph, &secret_node, &field.provenance);
            }
        }
    }

    for view in &document.application.views {
        let view_node = graph.add_node("View", view, None, BTreeMap::new());
        if let Some(application) = &application {
            graph.add_edge("contains", application, &view_node, BTreeMap::new());
        }
        attach_provenance(&mut graph, &view_node, format!("application.view:{view}"));
    }

    for tool in document.tools.values() {
        let tool_node = graph.add_node("Tool", &tool.name, None, attr(&[("label", &tool.label)]));
        if let Some(application) = &application {
            graph.add_edge("contains", application, &tool_node, BTreeMap::new());
        }
        attach_provenance(&mut graph, &tool_node, &tool.provenance);
        for input in tool.inputs.values() {
            let input_node = graph.add_node(
                "Input",
                format!("{}.{}", tool.name, input.name),
                Some(input.type_name.clone()),
                attr(&[("secret", if input.is_secret { "true" } else { "false" })]),
            );
            graph.add_edge("has_input", &tool_node, &input_node, BTreeMap::new());
            attach_provenance(&mut graph, &input_node, &input.provenance);
            if input.is_secret {
                let secret_node =
                    graph.add_node("Secret", input_node.name.clone(), None, BTreeMap::new());
                graph.add_edge(
                    "protects_secret",
                    &secret_node,
                    &input_node,
                    BTreeMap::new(),
                );
                attach_provenance(&mut graph, &secret_node, &input.provenance);
            }
        }
        for output in tool.outputs.values() {
            let output_node = graph.add_node(
                "Output",
                format!("{}.{}", tool.name, output.name),
                Some(output.type_name.clone()),
                attr(&[("secret", if output.is_secret { "true" } else { "false" })]),
            );
            graph.add_edge("has_output", &tool_node, &output_node, BTreeMap::new());
            attach_provenance(&mut graph, &output_node, &output.provenance);
            if output.is_secret {
                let secret_node =
                    graph.add_node("Secret", output_node.name.clone(), None, BTreeMap::new());
                graph.add_edge(
                    "protects_secret",
                    &secret_node,
                    &output_node,
                    BTreeMap::new(),
                );
                attach_provenance(&mut graph, &secret_node, &output.provenance);
            }
        }
        for requirement in &tool.requirements {
            let rule_node = graph.add_node("Rule", requirement, None, BTreeMap::new());
            graph.add_edge("requires", &tool_node, &rule_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &rule_node,
                format!("tool:{}.requirement:{requirement}", tool.name),
            );
        }
        for permission in &tool.permissions {
            let permission_node = graph.add_node("Permission", permission, None, BTreeMap::new());
            graph.add_edge("requires", &tool_node, &permission_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &permission_node,
                format!("tool:{}.permission:{permission}", tool.name),
            );
        }
        for approval in &tool.approvals {
            let approval_node = graph.add_node("Approval", approval, None, BTreeMap::new());
            graph.add_edge(
                "requires_approval",
                &tool_node,
                &approval_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &approval_node,
                format!("tool:{}.approval:{approval}", tool.name),
            );
        }
        for read in &tool.reads {
            let target = resolve_field_or_effect(&mut graph, document, read);
            let provenance = format!("tool:{}.read:{read}", tool.name);
            graph.add_edge(
                "reads",
                &tool_node,
                &target,
                attr(&[("provenance", &provenance)]),
            );
            if target.kind == "Effect" {
                attach_provenance(&mut graph, &target, provenance);
            }
        }
        for write in &tool.writes {
            let target = resolve_field_or_effect(&mut graph, document, write);
            let provenance = format!("tool:{}.write:{write}", tool.name);
            graph.add_edge(
                "writes",
                &tool_node,
                &target,
                attr(&[("provenance", &provenance)]),
            );
            if target.kind == "Effect" {
                attach_provenance(&mut graph, &target, provenance);
            }
        }
        for call in &tool.calls {
            let effect_node = graph.add_node("Effect", call, None, BTreeMap::new());
            graph.add_edge("calls", &tool_node, &effect_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &effect_node,
                format!("tool:{}.call:{call}", tool.name),
            );
        }
        for protection in &tool.secret_protections {
            let target = resolve_tool_secret_target(&mut graph, tool, protection);
            graph.add_edge("protects_secret", &tool_node, &target, BTreeMap::new());
        }
        for failure in &tool.failures {
            let failure_node = graph.add_node("Failure", failure, None, BTreeMap::new());
            let provenance = format!("tool:{}.failure:{failure}", tool.name);
            graph.add_edge(
                "may_fail_with",
                &tool_node,
                &failure_node,
                attr(&[("provenance", &provenance)]),
            );
            attach_provenance(&mut graph, &failure_node, provenance);
        }
        for guarantee in &tool.guarantees {
            let guarantee_node = graph.add_node("Guarantee", guarantee, None, BTreeMap::new());
            graph.add_edge("guarantees", &tool_node, &guarantee_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &guarantee_node,
                format!("tool:{}.guarantee:{guarantee}", tool.name),
            );
        }
        for trace in &tool.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge("records_trace", &tool_node, &trace_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("tool:{}.trace:{trace}", tool.name),
            );
        }
    }

    for pass in document.compiler_passes.values() {
        let pass_node = graph.add_node(
            "Action",
            &pass.name,
            None,
            attr(&[
                ("kind", "CompilerPass"),
                ("label", &pass.label),
                ("purpose", &pass.purpose),
            ]),
        );
        attach_provenance(&mut graph, &pass_node, &pass.provenance);
        for input in pass.inputs.values() {
            let value_node = graph.add_node(
                "Value",
                format!("{}.{}", pass.name, input.name),
                Some(input.type_name.clone()),
                BTreeMap::new(),
            );
            graph.add_edge("reads", &pass_node, &value_node, BTreeMap::new());
            attach_provenance(&mut graph, &value_node, &input.provenance);
        }
        for output in pass.outputs.values() {
            let value_node = graph.add_node(
                "Value",
                format!("{}.{}", pass.name, output.name),
                Some(output.type_name.clone()),
                BTreeMap::new(),
            );
            graph.add_edge("writes", &pass_node, &value_node, BTreeMap::new());
            attach_provenance(&mut graph, &value_node, &output.provenance);
        }
        for read in &pass.reads {
            let target = resolve_pass_value_or_effect(&mut graph, pass, read);
            let provenance = format!("compiler_pass:{}.read:{read}", pass.name);
            graph.add_edge(
                "reads",
                &pass_node,
                &target,
                attr(&[("provenance", &provenance)]),
            );
            if target.kind == "Effect" {
                attach_provenance(&mut graph, &target, provenance);
            }
        }
        for write in &pass.writes {
            let target = resolve_pass_value_or_effect(&mut graph, pass, write);
            let provenance = format!("compiler_pass:{}.write:{write}", pass.name);
            graph.add_edge(
                "writes",
                &pass_node,
                &target,
                attr(&[("provenance", &provenance)]),
            );
            if target.kind == "Effect" {
                attach_provenance(&mut graph, &target, provenance);
            }
        }
        for step in &pass.steps {
            let step_node = graph.add_node("Step", step, None, BTreeMap::new());
            graph.add_edge("contains", &pass_node, &step_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &step_node,
                format!("compiler_pass:{}.step:{step}", pass.name),
            );
        }
        for failure in &pass.failures {
            let failure_node = graph.add_node("Failure", failure, None, BTreeMap::new());
            let provenance = format!("compiler_pass:{}.failure:{failure}", pass.name);
            graph.add_edge(
                "may_fail_with",
                &pass_node,
                &failure_node,
                attr(&[("provenance", &provenance)]),
            );
            attach_provenance(&mut graph, &failure_node, provenance);
        }
        for guarantee in &pass.guarantees {
            let guarantee_node = graph.add_node("Guarantee", guarantee, None, BTreeMap::new());
            graph.add_edge("guarantees", &pass_node, &guarantee_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &guarantee_node,
                format!("compiler_pass:{}.guarantee:{guarantee}", pass.name),
            );
        }
        for trace in &pass.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge("records_trace", &pass_node, &trace_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("compiler_pass:{}.trace:{trace}", pass.name),
            );
        }
    }

    for component in document.system_components.values() {
        let component_node = graph.add_node(
            "SystemComponent",
            &component.name,
            None,
            attr(&[("label", &component.label)]),
        );
        if let Some(application) = &application {
            graph.add_edge("contains", application, &component_node, BTreeMap::new());
        }
        attach_provenance(&mut graph, &component_node, &component.provenance);
        for resource in component.resources.values() {
            let resource_node = graph.add_node(
                "Resource",
                format!("{}.{}", component.name, resource.name),
                Some(resource.type_name.clone()),
                BTreeMap::new(),
            );
            graph.add_edge(
                "uses_resource",
                &component_node,
                &resource_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &resource_node, &resource.provenance);
        }
        for owned_resource in &component.owned_resources {
            if let Some(resource_node) =
                resolve_system_component_resource(&graph, component, owned_resource)
            {
                graph.add_edge(
                    "owns_resource",
                    &component_node,
                    &resource_node,
                    BTreeMap::new(),
                );
            }
        }
        for borrowed_resource in &component.borrowed_resources {
            if let Some(resource_node) =
                resolve_system_component_resource(&graph, component, borrowed_resource)
            {
                graph.add_edge(
                    "borrows_resource",
                    &component_node,
                    &resource_node,
                    BTreeMap::new(),
                );
            }
        }
        for borrowed_resource in &component.mutably_borrowed_resources {
            if let Some(resource_node) =
                resolve_system_component_resource(&graph, component, borrowed_resource)
            {
                graph.add_edge(
                    "mutably_borrows_resource",
                    &component_node,
                    &resource_node,
                    BTreeMap::new(),
                );
            }
        }
        for placement in &component.resource_regions {
            if let Some(resource_node) =
                resolve_system_component_resource(&graph, component, &placement.resource_name)
            {
                let region_node = graph.add_node(
                    "Region",
                    format!("{}.{}", component.name, placement.region_name),
                    None,
                    BTreeMap::new(),
                );
                graph.add_edge(
                    "uses_region",
                    &component_node,
                    &region_node,
                    BTreeMap::new(),
                );
                graph.add_edge("in_region", &resource_node, &region_node, BTreeMap::new());
                attach_provenance(&mut graph, &region_node, &placement.provenance);
            }
        }
        for layout in &component.resource_layouts {
            let layout_node = graph.add_node(
                "Layout",
                format!("{}.{}", component.name, layout.resource_name),
                Some(layout.layout.clone()),
                attr(&[("resource", &layout.resource_name)]),
            );
            graph.add_edge(
                "uses_layout",
                &component_node,
                &layout_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &layout_node, &layout.provenance);
            if let Some(resource_node) =
                resolve_system_component_resource(&graph, component, &layout.resource_name)
            {
                graph.add_edge(
                    "layouts_resource",
                    &layout_node,
                    &resource_node,
                    BTreeMap::new(),
                );
            }
        }
        for allocation in &component.resource_allocations {
            let allocation_node = graph.add_node(
                "Allocation",
                format!("{}.{}", component.name, allocation.resource_name),
                Some(allocation.placement.clone()),
                attr(&[("resource", &allocation.resource_name)]),
            );
            graph.add_edge(
                "uses_allocation",
                &component_node,
                &allocation_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &allocation_node, &allocation.provenance);
            if let Some(resource_node) =
                resolve_system_component_resource(&graph, component, &allocation.resource_name)
            {
                graph.add_edge(
                    "allocates_resource",
                    &allocation_node,
                    &resource_node,
                    BTreeMap::new(),
                );
            }
        }
        for guard in &component.lock_guards {
            let guard_node = graph.add_node(
                "LockGuard",
                format!("{}.{}", component.name, guard.resource_name),
                Some(guard.lock_name.clone()),
                attr(&[
                    ("resource", &guard.resource_name),
                    ("lock", &guard.lock_name),
                ]),
            );
            graph.add_edge(
                "uses_lock_guard",
                &component_node,
                &guard_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &guard_node, &guard.provenance);
            if let Some(resource_node) =
                resolve_system_component_resource(&graph, component, &guard.resource_name)
            {
                graph.add_edge(
                    "guards_resource",
                    &guard_node,
                    &resource_node,
                    BTreeMap::new(),
                );
            }
            if let Some(lock_node) =
                resolve_system_component_resource(&graph, component, &guard.lock_name)
            {
                graph.add_edge(
                    "uses_lock_resource",
                    &guard_node,
                    &lock_node,
                    BTreeMap::new(),
                );
            }
        }
        for context in &component.execution_contexts {
            let context_node = graph.add_node(
                "ExecutionContext",
                format!("{}.{}", component.name, context.name),
                None,
                attr(&[("context", &context.name)]),
            );
            graph.add_edge(
                "runs_in_context",
                &component_node,
                &context_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &context_node, &context.provenance);
        }
        for priority in &component.interrupt_priorities {
            let priority_node = graph.add_node(
                "InterruptPriority",
                format!("{}.{}", component.name, priority.context_name),
                Some(priority.priority.clone()),
                attr(&[("context", &priority.context_name)]),
            );
            graph.add_edge(
                "uses_interrupt_priority",
                &component_node,
                &priority_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &priority_node, &priority.provenance);
            if let Some(context_node) = resolve_system_component_execution_context(
                &graph,
                component,
                &priority.context_name,
            ) {
                graph.add_edge(
                    "prioritizes_context",
                    &priority_node,
                    &context_node,
                    BTreeMap::new(),
                );
            }
        }
        for mask in &component.interrupt_masks {
            let mask_node = graph.add_node(
                "InterruptMask",
                format!("{}.{}", component.name, mask.context_name),
                Some(mask.mask.clone()),
                attr(&[("context", &mask.context_name)]),
            );
            graph.add_edge(
                "uses_interrupt_mask",
                &component_node,
                &mask_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &mask_node, &mask.provenance);
            if let Some(context_node) =
                resolve_system_component_execution_context(&graph, component, &mask.context_name)
            {
                graph.add_edge("masks_context", &mask_node, &context_node, BTreeMap::new());
            }
        }
        for task in &component.scheduler_tasks {
            let task_node = graph.add_node(
                "SchedulerTask",
                format!("{}.{}", component.name, task.task_name),
                Some(task.context_name.clone()),
                attr(&[("context", &task.context_name)]),
            );
            graph.add_edge(
                "schedules_task",
                &component_node,
                &task_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &task_node, &task.provenance);
            if let Some(context_node) =
                resolve_system_component_execution_context(&graph, component, &task.context_name)
            {
                graph.add_edge(
                    "task_runs_in_context",
                    &task_node,
                    &context_node,
                    BTreeMap::new(),
                );
            }
        }
        for priority in &component.scheduler_task_priorities {
            let priority_node = graph.add_node(
                "SchedulerTaskPriority",
                format!("{}.{}", component.name, priority.task_name),
                Some(priority.priority.clone()),
                attr(&[("task", &priority.task_name)]),
            );
            graph.add_edge(
                "uses_task_priority",
                &component_node,
                &priority_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &priority_node, &priority.provenance);
            if let Some(task_node) =
                resolve_system_component_scheduler_task(&graph, component, &priority.task_name)
            {
                graph.add_edge(
                    "prioritizes_task",
                    &priority_node,
                    &task_node,
                    BTreeMap::new(),
                );
            }
        }
        for timing in &component.scheduler_task_timings {
            let timing_node = graph.add_node(
                "SchedulerTaskTiming",
                format!("{}.{}", component.name, timing.task_name),
                Some(format!(
                    "deadline {}, budget {}",
                    timing.deadline, timing.budget
                )),
                attr(&[
                    ("task", &timing.task_name),
                    ("deadline", &timing.deadline),
                    ("budget", &timing.budget),
                ]),
            );
            graph.add_edge(
                "uses_task_timing",
                &component_node,
                &timing_node,
                BTreeMap::new(),
            );
            attach_provenance(&mut graph, &timing_node, &timing.provenance);
            if let Some(task_node) =
                resolve_system_component_scheduler_task(&graph, component, &timing.task_name)
            {
                graph.add_edge("times_task", &timing_node, &task_node, BTreeMap::new());
            }
        }
        for capability in &component.capabilities {
            let capability_node = graph.add_node("Capability", capability, None, BTreeMap::new());
            graph.add_edge(
                "requires",
                &component_node,
                &capability_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &capability_node,
                format!(
                    "system_component:{}.capability:{capability}",
                    component.name
                ),
            );
            if let Some(resource_node) =
                resolve_system_capability_resource(&graph, component, capability)
            {
                graph.add_edge(
                    "authorizes_resource",
                    &capability_node,
                    &resource_node,
                    BTreeMap::new(),
                );
            }
        }
        for effect in &component.effects {
            let effect_node = graph.add_node("Effect", effect, None, BTreeMap::new());
            let provenance = format!("system_component:{}.effect:{effect}", component.name);
            graph.add_edge(
                "performs",
                &component_node,
                &effect_node,
                attr(&[("provenance", &provenance)]),
            );
            attach_provenance(&mut graph, &effect_node, provenance);
            if let Some(resource_node) = resolve_system_effect_resource(&graph, component, effect) {
                graph.add_edge(
                    "targets_resource",
                    &effect_node,
                    &resource_node,
                    BTreeMap::new(),
                );
            }
        }
        for guarantee in &component.guarantees {
            let guarantee_node = graph.add_node("Guarantee", guarantee, None, BTreeMap::new());
            graph.add_edge(
                "guarantees",
                &component_node,
                &guarantee_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &guarantee_node,
                format!("system_component:{}.guarantee:{guarantee}", component.name),
            );
        }
        for trace in &component.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge(
                "records_trace",
                &component_node,
                &trace_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("system_component:{}.trace:{trace}", component.name),
            );
        }
    }

    for action in document.actions.values() {
        let action_node = graph.add_node(
            "Action",
            &action.name,
            None,
            attr(&[("label", &action.label), ("trigger", &action.trigger)]),
        );
        if let Some(application) = &application {
            graph.add_edge("contains", application, &action_node, BTreeMap::new());
        }
        attach_provenance(&mut graph, &action_node, &action.provenance);
        for requirement in &action.requirements {
            let rule_node = graph.add_node("Rule", requirement, None, BTreeMap::new());
            graph.add_edge("requires", &action_node, &rule_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &rule_node,
                format!("action:{}.requirement:{requirement}", action.name),
            );
        }
        for write in &action.writes {
            let target = resolve_field_or_effect(&mut graph, document, write);
            let provenance = format!("action:{}.write:{write}", action.name);
            graph.add_edge(
                "writes",
                &action_node,
                &target,
                attr(&[("provenance", &provenance)]),
            );
            if target.kind == "Effect" {
                attach_provenance(&mut graph, &target, provenance);
            }
        }
        for read in &action.reads {
            let target = resolve_field_or_effect(&mut graph, document, read);
            let provenance = format!("action:{}.read:{read}", action.name);
            graph.add_edge(
                "reads",
                &action_node,
                &target,
                attr(&[("provenance", &provenance)]),
            );
            if target.kind == "Effect" {
                attach_provenance(&mut graph, &target, provenance);
            }
        }
        for protection in &action.secret_protections {
            let target = resolve_secret_target(&mut graph, document, protection);
            graph.add_edge("protects_secret", &action_node, &target, BTreeMap::new());
            if target.kind == "Effect" {
                attach_provenance(
                    &mut graph,
                    &target,
                    format!("action:{}.secret_protection:{protection}", action.name),
                );
            }
        }
        for failure in &action.failures {
            let failure_node = graph.add_node("Failure", failure, None, BTreeMap::new());
            let provenance = format!("action:{}.failure:{failure}", action.name);
            graph.add_edge(
                "may_fail_with",
                &action_node,
                &failure_node,
                attr(&[("provenance", &provenance)]),
            );
            attach_provenance(&mut graph, &failure_node, provenance);
        }
        for guarantee in &action.guarantees {
            let guarantee_node = graph.add_node("Guarantee", guarantee, None, BTreeMap::new());
            graph.add_edge("guarantees", &action_node, &guarantee_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &guarantee_node,
                format!("action:{}.guarantee:{guarantee}", action.name),
            );
        }
        for trace in &action.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge("records_trace", &action_node, &trace_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("action:{}.trace:{trace}", action.name),
            );
        }
    }

    for failure in document.failures.values() {
        let failure_node = graph.add_node(
            "Failure",
            &failure.name,
            None,
            attr(&[("condition", &failure.condition), ("declared", "true")]),
        );
        set_graph_node_attribute(&mut graph, &failure_node.id, "declared", "true");
        set_graph_node_attribute(
            &mut graph,
            &failure_node.id,
            "condition",
            &failure.condition,
        );
        set_graph_node_attribute(
            &mut graph,
            &failure_node.id,
            "provenance",
            &failure.provenance,
        );
        let provenance = graph.add_node("Provenance", &failure.provenance, None, BTreeMap::new());
        graph.add_edge(
            "has_provenance",
            &failure_node,
            &provenance,
            BTreeMap::new(),
        );
        for handling in &failure.handling {
            let handling_node = graph.add_node("Effect", handling, None, BTreeMap::new());
            graph.add_edge(
                "handles_failure",
                &failure_node,
                &handling_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &handling_node,
                format!("failure:{}.handling:{handling}", failure.name),
            );
        }
        for trace in &failure.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge("records_trace", &failure_node, &trace_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("failure:{}.trace:{trace}", failure.name),
            );
        }
    }

    AilCore {
        package: package.metadata.clone(),
        graph,
    }
}

fn attach_provenance(graph: &mut Graph, source: &Node, provenance: impl AsRef<str>) {
    let provenance = graph.add_node("Provenance", provenance.as_ref(), None, BTreeMap::new());
    graph.add_edge("has_provenance", source, &provenance, BTreeMap::new());
}

pub fn check_ail_core(core: &AilCore) -> Vec<String> {
    check_ail_core_diagnostics(core)
        .into_iter()
        .map(|diagnostic| diagnostic.to_string())
        .collect()
}

pub fn check_ail_core_diagnostics(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = core
        .graph
        .validate_edge_references()
        .into_iter()
        .map(AilDiagnostic::from_message)
        .collect::<Vec<_>>();
    diagnostics.extend(check_core_schema_catalog(core));
    diagnostics.extend(check_field_types(core));
    diagnostics.extend(check_requirement_reference_diagnostics(core));
    diagnostics.extend(check_requirement_field_references(core));
    diagnostics.extend(check_action_failure_declarations(core));
    diagnostics.extend(check_secret_write_protection(core));
    diagnostics.extend(check_secret_read_protection(core));
    diagnostics.extend(check_tool_secret_output_disclosure(core));
    diagnostics.extend(check_unknown_field_references(core));
    diagnostics.extend(check_failure_handling(core));
    diagnostics.extend(check_failure_trace_coverage(core));
    diagnostics.extend(check_semantic_node_provenance(core));
    diagnostics.extend(check_guarantee_attachment(core));
    diagnostics.extend(check_trace_attachment(core));
    diagnostics.extend(check_rule_attachment(core));
    diagnostics.extend(check_effect_attachment(core));
    diagnostics.extend(check_secret_attachment(core));
    diagnostics.extend(check_tool_trace_coverage(core));
    diagnostics.extend(check_tool_approval_mentions(core));
    diagnostics.extend(check_tool_permission_mentions(core));
    diagnostics.extend(check_system_effect_capabilities(core));
    diagnostics.extend(check_system_effect_resources(core));
    diagnostics.extend(check_system_device_effect_capabilities(core));
    diagnostics.extend(check_system_layout_resources(core));
    diagnostics.extend(check_system_allocation_resources(core));
    diagnostics.extend(check_system_lock_guards(core));
    diagnostics.extend(check_system_interrupt_priority_contexts(core));
    diagnostics.extend(check_system_interrupt_mask_contexts(core));
    diagnostics.extend(check_system_scheduler_task_contexts(core));
    diagnostics.extend(check_system_scheduler_task_priorities(core));
    diagnostics.extend(check_system_scheduler_task_timings(core));
    diagnostics.extend(check_system_interrupt_context_effects(core));
    diagnostics.extend(check_system_mutable_effect_ownership(core));
    diagnostics.extend(check_system_shared_mutable_borrow_conflicts(core));
    diagnostics.extend(check_system_mutable_borrow_conflicts(core));
    diagnostics.extend(check_system_read_effect_borrowing(core));
    diagnostics.extend(check_system_effect_resource_regions(core));
    diagnostics.extend(check_system_use_after_release(core));
    diagnostics.extend(check_system_use_after_move(core));
    for action in core.graph.nodes.iter().filter(|node| node.kind == "Action") {
        if !has_outgoing_edge(&core.graph, "records_trace", &action.id) {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL010",
                    format!("action {} is missing trace coverage", action.name),
                )
                .with_source_provenance(node_provenance(core, &action.id))
                .with_affected_graph_item(format!("node:{}", action.id))
                .with_repair_suggestion(format!("Add a trace bullet to action {}.", action.name)),
            );
        }
    }
    diagnostics.sort_by_key(|diagnostic| diagnostic.to_string());
    diagnostics
}

fn check_core_schema_catalog(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for node in &core.graph.nodes {
        if !is_known_core_node_kind(&node.kind) {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-SCHEMA-001",
                    format!("unknown AIL-Core node kind '{}'", node.kind),
                )
                .with_source_provenance(node_provenance(core, &node.id))
                .with_affected_graph_item(format!("node:{}", node.id))
                .with_repair_suggestion(format!(
                    "Use a node kind declared in ail-core.schema.v0 instead of '{}'.",
                    node.kind
                )),
            );
        }
    }
    for edge in &core.graph.edges {
        if !is_known_core_edge_kind(&edge.kind) {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-SCHEMA-002",
                    format!("unknown AIL-Core edge kind '{}'", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id))
                .with_repair_suggestion(format!(
                    "Use an edge kind declared in ail-core.schema.v0 instead of '{}'.",
                    edge.kind
                )),
            );
        }
    }
    diagnostics
}

fn is_known_core_node_kind(kind: &str) -> bool {
    matches!(
        kind,
        "Action"
            | "Allocation"
            | "Application"
            | "Approval"
            | "Branch"
            | "Call"
            | "Capability"
            | "CorpusFixture"
            | "Diagnostic"
            | "Effect"
            | "Event"
            | "ExecutionContext"
            | "ExternalBinding"
            | "Failure"
            | "Field"
            | "Function"
            | "Guarantee"
            | "Input"
            | "InterruptMask"
            | "InterruptPriority"
            | "Layout"
            | "LockGuard"
            | "Loop"
            | "Lowering"
            | "Match"
            | "Output"
            | "Package"
            | "Permission"
            | "Prompt"
            | "Provenance"
            | "Region"
            | "Resource"
            | "Return"
            | "Rule"
            | "SchedulerTask"
            | "SchedulerTaskPriority"
            | "SchedulerTaskTiming"
            | "Secret"
            | "Step"
            | "SystemComponent"
            | "Thing"
            | "Tool"
            | "Trace"
            | "User"
            | "Value"
            | "View"
    )
}

fn is_known_core_edge_kind(kind: &str) -> bool {
    matches!(
        kind,
        "allocates_resource"
            | "authorizes_resource"
            | "borrows_resource"
            | "calls"
            | "contains"
            | "depends_on"
            | "emits"
            | "grants_permission"
            | "guards_resource"
            | "guarantees"
            | "handles_failure"
            | "has_field"
            | "has_input"
            | "has_output"
            | "has_provenance"
            | "in_region"
            | "layouts_resource"
            | "lowers_to"
            | "masks_context"
            | "may_fail_with"
            | "mutably_borrows_resource"
            | "owns_resource"
            | "performs"
            | "prioritizes_context"
            | "prioritizes_task"
            | "projects_to"
            | "protects_secret"
            | "reads"
            | "records_trace"
            | "requires"
            | "requires_approval"
            | "runs_in_context"
            | "schedules_task"
            | "targets_resource"
            | "task_runs_in_context"
            | "times_task"
            | "uses_allocation"
            | "uses_interrupt_mask"
            | "uses_interrupt_priority"
            | "uses_layout"
            | "uses_lock_guard"
            | "uses_lock_resource"
            | "uses_region"
            | "uses_resource"
            | "uses_task_priority"
            | "uses_task_timing"
            | "writes"
    )
}

pub fn render_ail_core(core: &AilCore) -> String {
    let mut lines = vec![
        format!("package: {}", core.package.name),
        format!("version: {}", core.package.version),
        format!("profile: {}", core.package.profile),
        format!("entry: {}", core.package.entry),
        format!("features: {}", core.package.features.join(", ")),
        format!("imports: {}", render_import_specs(&core.package.imports)),
        format!("conformance: {}", core.package.conformance),
        format!("base_llm_endpoint: {}", core.package.base_llm_endpoint),
        String::new(),
        "nodes:".to_string(),
    ];
    let mut nodes = core.graph.nodes.clone();
    nodes.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then(left.name.cmp(&right.name))
            .then(left.id.cmp(&right.id))
    });
    for node in &nodes {
        let mut line = format!("node {} {}", node.kind, node.name);
        if let Some(type_name) = &node.type_name {
            line.push_str(&format!(" : {type_name}"));
        }
        if !node.attributes.is_empty() {
            line.push_str(&format!(" [{}]", render_core_attributes(&node.attributes)));
        }
        lines.push(line);
    }
    lines.push(String::new());
    lines.push("edges:".to_string());
    let node_labels = core
        .graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), format!("{}:{}", node.kind, node.name)))
        .collect::<BTreeMap<_, _>>();
    for edge in &core.graph.edges {
        let source = node_labels
            .get(&edge.source)
            .map(String::as_str)
            .unwrap_or(edge.source.as_str());
        let target = node_labels
            .get(&edge.target)
            .map(String::as_str)
            .unwrap_or(edge.target.as_str());
        let mut line = format!("edge {} {} -> {}", edge.kind, source, target);
        if !edge.attributes.is_empty() {
            line.push_str(&format!(" [{}]", render_core_attributes(&edge.attributes)));
        }
        lines.push(line);
    }
    lines.join("\n")
}

fn render_import_specs(imports: &[AilImportSpec]) -> String {
    imports
        .iter()
        .map(|import| format!("{} as {}", import.path, import.alias))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn parse_ail_core_text(text: &str) -> Result<AilCore, String> {
    let mut package_name = None;
    let mut package_version = None;
    let mut profile = None;
    let mut entry = None;
    let mut features = Vec::new();
    let mut imports = Vec::new();
    let mut conformance = None;
    let mut base_llm_endpoint = None;
    let mut section = "";
    let mut graph = Graph::default();
    let mut node_by_label: BTreeMap<String, Node> = BTreeMap::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "nodes:" {
            section = "nodes";
            continue;
        }
        if line == "edges:" {
            section = "edges";
            continue;
        }

        match section {
            "" => {
                let Some((key, value)) = line.split_once(':') else {
                    return Err(format!("AIL-Core line {line_number}: expected metadata"));
                };
                let value = value.trim().to_string();
                match key.trim() {
                    "package" => package_name = Some(value),
                    "version" => package_version = Some(value),
                    "profile" => profile = Some(value),
                    "entry" => entry = Some(value),
                    "features" => {
                        features = value
                            .split(',')
                            .map(str::trim)
                            .filter(|feature| !feature.is_empty())
                            .map(ToString::to_string)
                            .collect();
                    }
                    "imports" => {
                        imports = if value.is_empty() {
                            Vec::new()
                        } else {
                            parse_import_specs(&value)?
                        };
                    }
                    "conformance" => conformance = Some(value),
                    "base_llm_endpoint" => base_llm_endpoint = Some(value),
                    key => {
                        return Err(format!(
                            "AIL-Core line {line_number}: unknown metadata key '{key}'"
                        ));
                    }
                }
            }
            "nodes" => {
                let parsed = parse_core_node_line(line, line_number)?;
                let node = graph.add_node(
                    parsed.kind,
                    parsed.name,
                    parsed.type_name,
                    parsed.attributes,
                );
                let label = core_node_label(&node);
                if node_by_label.insert(label.clone(), node).is_some() {
                    return Err(format!(
                        "AIL-Core line {line_number}: duplicate node label {label}"
                    ));
                }
            }
            "edges" => {
                let parsed = parse_core_edge_line(line, line_number)?;
                let source = node_by_label
                    .get(&parsed.source_label)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "AIL-Core line {line_number}: unknown source node {}",
                            parsed.source_label
                        )
                    })?;
                let target = node_by_label
                    .get(&parsed.target_label)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "AIL-Core line {line_number}: unknown target node {}",
                            parsed.target_label
                        )
                    })?;
                graph.add_edge(parsed.kind, &source, &target, parsed.attributes);
            }
            _ => unreachable!("AIL-Core parser only sets known sections"),
        }
    }

    let missing_references = graph.validate_edge_references();
    if !missing_references.is_empty() {
        return Err(format!(
            "AIL-Core graph has missing edge references: {}",
            missing_references.join("; ")
        ));
    }

    Ok(AilCore {
        package: AilPackageMetadata {
            name: package_name.ok_or_else(|| "AIL-Core missing package metadata".to_string())?,
            version: package_version
                .ok_or_else(|| "AIL-Core missing version metadata".to_string())?,
            profile: profile.ok_or_else(|| "AIL-Core missing profile metadata".to_string())?,
            entry: entry.unwrap_or_default(),
            features,
            imports,
            conformance: conformance
                .ok_or_else(|| "AIL-Core missing conformance metadata".to_string())?,
            base_llm_endpoint: base_llm_endpoint
                .ok_or_else(|| "AIL-Core missing base_llm_endpoint metadata".to_string())?,
        },
        graph,
    })
}

fn render_core_attributes(attributes: &BTreeMap<String, String>) -> String {
    attributes
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(",")
}

struct ParsedCoreNode {
    kind: String,
    name: String,
    type_name: Option<String>,
    attributes: BTreeMap<String, String>,
}

struct ParsedCoreEdge {
    kind: String,
    source_label: String,
    target_label: String,
    attributes: BTreeMap<String, String>,
}

fn parse_core_node_line(line: &str, line_number: usize) -> Result<ParsedCoreNode, String> {
    let Some(rest) = line.strip_prefix("node ") else {
        return Err(format!("AIL-Core line {line_number}: expected node line"));
    };
    let Some((kind, rest)) = rest.split_once(' ') else {
        return Err(format!(
            "AIL-Core line {line_number}: node is missing a name"
        ));
    };
    let (rest, attributes) = parse_core_attribute_suffix(rest, line_number)?;
    let (name, type_name) = if let Some((name, type_name)) = rest.rsplit_once(" : ") {
        (name.trim().to_string(), Some(type_name.trim().to_string()))
    } else {
        (rest.trim().to_string(), None)
    };
    if name.is_empty() {
        return Err(format!("AIL-Core line {line_number}: node name is empty"));
    }
    Ok(ParsedCoreNode {
        kind: kind.to_string(),
        name,
        type_name,
        attributes,
    })
}

fn parse_core_edge_line(line: &str, line_number: usize) -> Result<ParsedCoreEdge, String> {
    let Some(rest) = line.strip_prefix("edge ") else {
        return Err(format!("AIL-Core line {line_number}: expected edge line"));
    };
    let Some((kind, rest)) = rest.split_once(' ') else {
        return Err(format!(
            "AIL-Core line {line_number}: edge is missing endpoints"
        ));
    };
    let (rest, attributes) = parse_core_attribute_suffix(rest, line_number)?;
    let Some((source, target)) = rest.split_once(" -> ") else {
        return Err(format!(
            "AIL-Core line {line_number}: edge is missing ' -> ' endpoint separator"
        ));
    };
    Ok(ParsedCoreEdge {
        kind: kind.to_string(),
        source_label: source.trim().to_string(),
        target_label: target.trim().to_string(),
        attributes,
    })
}

fn parse_core_attribute_suffix(
    text: &str,
    line_number: usize,
) -> Result<(&str, BTreeMap<String, String>), String> {
    if !text.ends_with(']') {
        return Ok((text, BTreeMap::new()));
    }
    let Some((prefix, attributes)) = text.rsplit_once(" [") else {
        return Err(format!(
            "AIL-Core line {line_number}: malformed attribute suffix"
        ));
    };
    let attributes = attributes
        .strip_suffix(']')
        .ok_or_else(|| format!("AIL-Core line {line_number}: malformed attribute suffix"))?;
    Ok((prefix, parse_core_attributes(attributes, line_number)?))
}

fn parse_core_attributes(
    text: &str,
    line_number: usize,
) -> Result<BTreeMap<String, String>, String> {
    let mut attributes = BTreeMap::new();
    if text.trim().is_empty() {
        return Ok(attributes);
    }
    for entry in split_core_attribute_entries(text) {
        let Some((key, value)) = entry.split_once('=') else {
            return Err(format!(
                "AIL-Core line {line_number}: malformed attribute '{entry}'"
            ));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(format!(
                "AIL-Core line {line_number}: attribute key is empty"
            ));
        }
        attributes.insert(key.to_string(), value.trim().to_string());
    }
    Ok(attributes)
}

fn split_core_attribute_entries(text: &str) -> Vec<&str> {
    let mut entries = Vec::new();
    let mut start = 0;
    for (index, ch) in text.char_indices() {
        if ch == ',' && starts_core_attribute_key(&text[index + ch.len_utf8()..]) {
            entries.push(text[start..index].trim());
            start = index + ch.len_utf8();
        }
    }
    entries.push(text[start..].trim());
    entries
}

fn starts_core_attribute_key(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    for ch in chars {
        if ch == '=' {
            return true;
        }
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            return false;
        }
    }
    false
}

fn core_node_label(node: &Node) -> String {
    format!("{}:{}", node.kind, node.name)
}

fn flow_core_label(core: &AilCore, kind: &str, name: &str) -> String {
    core.graph
        .find_node(kind, name)
        .map(core_node_label)
        .unwrap_or_else(|| format!("{kind}:{name}"))
}

pub fn render_ail_flow_view(core: &AilCore) -> String {
    let application = sorted_node_names(core, "Application")
        .into_iter()
        .next()
        .unwrap_or_default();
    let things = sorted_node_names(core, "Thing")
        .into_iter()
        .map(|thing| render_flow_thing(core, &thing))
        .collect::<Vec<_>>()
        .join(",");
    let views = render_json_array(sorted_node_names(core, "View"));
    let actions = sorted_action_names(core)
        .into_iter()
        .map(|action| render_flow_action(core, &action))
        .collect::<Vec<_>>()
        .join(",");
    let tools = sorted_node_names(core, "Tool")
        .into_iter()
        .map(|tool| render_flow_tool(core, &tool))
        .collect::<Vec<_>>()
        .join(",");
    let compiler_passes = core
        .graph
        .nodes
        .iter()
        .filter(|node| {
            node.kind == "Action"
                && node
                    .attributes
                    .get("kind")
                    .is_some_and(|kind| kind == "CompilerPass")
        })
        .map(|node| render_flow_compiler_pass(core, &node.name))
        .collect::<Vec<_>>()
        .join(",");
    let system_components = sorted_node_names(core, "SystemComponent")
        .into_iter()
        .map(|component| render_flow_system_component(core, &component))
        .collect::<Vec<_>>()
        .join(",");
    let core_hash = ail_core_hash(core);

    format!(
        "{{\"kind\":\"AIL-Flow\",\"package\":{},\"coreHash\":{},\"application\":{},\"things\":[{}],\"views\":{},\"actions\":[{}],\"tools\":[{}],\"compilerPasses\":[{}],\"systemComponents\":[{}]}}",
        json_string(&core.package.name),
        json_string(&core_hash),
        json_string(&application),
        things,
        views,
        actions,
        tools,
        compiler_passes,
        system_components
    )
}

fn render_flow_thing(core: &AilCore, thing: &str) -> String {
    let field_prefix = format!("{thing}.");
    let mut fields = core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Field" && node.name.starts_with(&field_prefix))
        .map(|field| {
            let field_name = field
                .name
                .strip_prefix(&field_prefix)
                .unwrap_or(field.name.as_str());
            let secret = field
                .attributes
                .get("secret")
                .is_some_and(|value| value == "true");
            format!(
                "{{\"name\":{},\"coreLabel\":{},\"type\":{},\"secret\":{}}}",
                json_string(field_name),
                json_string(&core_node_label(field)),
                json_string(field.type_name.as_deref().unwrap_or("")),
                secret
            )
        })
        .collect::<Vec<_>>();
    fields.sort();
    let core_label = flow_core_label(core, "Thing", thing);
    format!(
        "{{\"name\":{},\"coreLabel\":{},\"fields\":[{}]}}",
        json_string(thing),
        json_string(&core_label),
        fields.join(",")
    )
}

fn render_flow_action(core: &AilCore, action: &str) -> String {
    let Some(action_node) = core.graph.find_node("Action", action) else {
        let core_label = flow_core_label(core, "Action", action);
        return format!(
            "{{\"name\":{},\"coreLabel\":{},\"label\":\"\",\"trigger\":\"\",\"requires\":[],\"reads\":[],\"writes\":[],\"guarantees\":[],\"traces\":[],\"edgeRefs\":[]}}",
            json_string(action),
            json_string(&core_label)
        );
    };
    let label = action_node
        .attributes
        .get("label")
        .map(String::as_str)
        .unwrap_or("");
    let trigger = action_node
        .attributes
        .get("trigger")
        .map(String::as_str)
        .unwrap_or("");
    format!(
        "{{\"name\":{},\"coreLabel\":{},\"label\":{},\"trigger\":{},\"requires\":{},\"reads\":{},\"writes\":{},\"guarantees\":{},\"traces\":{},\"edgeRefs\":{}}}",
        json_string(action),
        json_string(&core_node_label(action_node)),
        json_string(label),
        json_string(trigger),
        render_json_array(edge_target_names(core, &action_node.id, "requires")),
        render_json_array(edge_target_names(core, &action_node.id, "reads")),
        render_json_array(edge_target_names(core, &action_node.id, "writes")),
        render_json_array(edge_target_names(core, &action_node.id, "guarantees")),
        render_json_array(edge_target_names(core, &action_node.id, "records_trace")),
        render_flow_edge_refs(
            core,
            action_node,
            &["requires", "reads", "writes", "guarantees", "records_trace"]
        ),
    )
}

fn render_flow_tool(core: &AilCore, tool: &str) -> String {
    let Some(tool_node) = core.graph.find_node("Tool", tool) else {
        let core_label = flow_core_label(core, "Tool", tool);
        return format!(
            "{{\"name\":{},\"coreLabel\":{},\"label\":\"\",\"requires\":[],\"inputs\":[],\"outputs\":[],\"reads\":[],\"writes\":[],\"calls\":[],\"permissions\":[],\"approvals\":[],\"guarantees\":[],\"traces\":[],\"edgeRefs\":[]}}",
            json_string(tool),
            json_string(&core_label)
        );
    };
    let label = tool_node
        .attributes
        .get("label")
        .map(String::as_str)
        .unwrap_or("");
    format!(
        "{{\"name\":{},\"coreLabel\":{},\"label\":{},\"requires\":{},\"inputs\":{},\"outputs\":{},\"reads\":{},\"writes\":{},\"calls\":{},\"permissions\":{},\"approvals\":{},\"guarantees\":{},\"traces\":{},\"edgeRefs\":{}}}",
        json_string(tool),
        json_string(&core_node_label(tool_node)),
        json_string(label),
        render_json_array(edge_target_names(core, &tool_node.id, "requires")),
        render_json_array(edge_target_names(core, &tool_node.id, "has_input")),
        render_json_array(edge_target_names(core, &tool_node.id, "has_output")),
        render_json_array(edge_target_names(core, &tool_node.id, "reads")),
        render_json_array(edge_target_names(core, &tool_node.id, "writes")),
        render_json_array(edge_target_names(core, &tool_node.id, "calls")),
        render_json_array(edge_target_kind_names(
            core,
            &tool_node.id,
            "requires",
            "Permission"
        )),
        render_json_array(edge_target_names(core, &tool_node.id, "requires_approval")),
        render_json_array(edge_target_names(core, &tool_node.id, "guarantees")),
        render_json_array(edge_target_names(core, &tool_node.id, "records_trace")),
        render_flow_edge_refs(
            core,
            tool_node,
            &[
                "requires",
                "has_input",
                "has_output",
                "reads",
                "writes",
                "calls",
                "requires_approval",
                "protects_secret",
                "guarantees",
                "records_trace"
            ]
        ),
    )
}

fn render_flow_compiler_pass(core: &AilCore, pass: &str) -> String {
    let Some(pass_node) = core.graph.find_node("Action", pass) else {
        let core_label = flow_core_label(core, "Action", pass);
        return format!(
            "{{\"name\":{},\"coreLabel\":{},\"label\":\"\",\"inputs\":[],\"outputs\":[],\"reads\":[],\"writes\":[],\"steps\":[],\"guarantees\":[],\"traces\":[],\"edgeRefs\":[]}}",
            json_string(pass),
            json_string(&core_label)
        );
    };
    let label = pass_node
        .attributes
        .get("label")
        .map(String::as_str)
        .unwrap_or("");
    format!(
        "{{\"name\":{},\"coreLabel\":{},\"label\":{},\"inputs\":{},\"outputs\":{},\"reads\":{},\"writes\":{},\"steps\":{},\"guarantees\":{},\"traces\":{},\"edgeRefs\":{}}}",
        json_string(pass),
        json_string(&core_node_label(pass_node)),
        json_string(label),
        render_json_array(
            edge_target_names(core, &pass_node.id, "reads")
                .into_iter()
                .filter(|name| name.starts_with(&format!("{pass}.")))
                .collect()
        ),
        render_json_array(
            edge_target_names(core, &pass_node.id, "writes")
                .into_iter()
                .filter(|name| name.starts_with(&format!("{pass}.")))
                .collect()
        ),
        render_json_array(edge_target_names(core, &pass_node.id, "reads")),
        render_json_array(edge_target_names(core, &pass_node.id, "writes")),
        render_json_array(edge_target_names(core, &pass_node.id, "contains")),
        render_json_array(edge_target_names(core, &pass_node.id, "guarantees")),
        render_json_array(edge_target_names(core, &pass_node.id, "records_trace")),
        render_flow_edge_refs(
            core,
            pass_node,
            &[
                "reads",
                "writes",
                "contains",
                "may_fail_with",
                "guarantees",
                "records_trace"
            ]
        ),
    )
}

fn render_flow_system_component(core: &AilCore, component: &str) -> String {
    let Some(component_node) = core.graph.find_node("SystemComponent", component) else {
        let core_label = flow_core_label(core, "SystemComponent", component);
        return format!(
            "{{\"name\":{},\"coreLabel\":{},\"label\":\"\",\"resources\":[],\"owns\":[],\"borrows\":[],\"mutablyBorrows\":[],\"regions\":[],\"layouts\":[],\"allocations\":[],\"lockGuards\":[],\"contexts\":[],\"priorities\":[],\"interruptMasks\":[],\"tasks\":[],\"taskPriorities\":[],\"taskTimings\":[],\"capabilities\":[],\"effects\":[],\"guarantees\":[],\"traces\":[],\"edgeRefs\":[]}}",
            json_string(component),
            json_string(&core_label)
        );
    };
    let label = component_node
        .attributes
        .get("label")
        .map(String::as_str)
        .unwrap_or("");
    format!(
        "{{\"name\":{},\"coreLabel\":{},\"label\":{},\"resources\":{},\"owns\":{},\"borrows\":{},\"mutablyBorrows\":{},\"regions\":{},\"layouts\":{},\"allocations\":{},\"lockGuards\":{},\"contexts\":{},\"priorities\":{},\"interruptMasks\":{},\"tasks\":{},\"taskPriorities\":{},\"taskTimings\":{},\"capabilities\":{},\"effects\":{},\"guarantees\":{},\"traces\":{},\"edgeRefs\":{}}}",
        json_string(component),
        json_string(&core_node_label(component_node)),
        json_string(label),
        render_json_array(edge_target_names(core, &component_node.id, "uses_resource")),
        render_json_array(edge_target_names(core, &component_node.id, "owns_resource")),
        render_json_array(edge_target_names(
            core,
            &component_node.id,
            "borrows_resource",
        )),
        render_json_array(edge_target_names(
            core,
            &component_node.id,
            "mutably_borrows_resource",
        )),
        render_json_array(edge_target_names(core, &component_node.id, "uses_region")),
        render_json_array(system_component_layout_summaries(core, &component_node.id)),
        render_json_array(system_component_allocation_summaries(
            core,
            &component_node.id
        )),
        render_json_array(system_component_lock_guard_summaries(
            core,
            &component_node.id
        )),
        render_json_array(edge_target_names(
            core,
            &component_node.id,
            "runs_in_context",
        )),
        render_json_array(system_component_interrupt_priority_summaries(
            core,
            &component_node.id
        )),
        render_json_array(system_component_interrupt_mask_summaries(
            core,
            &component_node.id
        )),
        render_json_array(system_component_scheduler_task_summaries(
            core,
            &component_node.id
        )),
        render_json_array(system_component_scheduler_task_priority_summaries(
            core,
            &component_node.id
        )),
        render_json_array(system_component_scheduler_task_timing_summaries(
            core,
            &component_node.id
        )),
        render_json_array(edge_target_names(core, &component_node.id, "requires")),
        render_json_array(edge_target_names(core, &component_node.id, "performs")),
        render_json_array(edge_target_names(core, &component_node.id, "guarantees")),
        render_json_array(edge_target_names(core, &component_node.id, "records_trace")),
        render_flow_edge_refs(
            core,
            component_node,
            &[
                "uses_resource",
                "owns_resource",
                "borrows_resource",
                "mutably_borrows_resource",
                "uses_region",
                "uses_layout",
                "uses_allocation",
                "uses_lock_guard",
                "runs_in_context",
                "uses_interrupt_priority",
                "uses_interrupt_mask",
                "schedules_task",
                "uses_task_priority",
                "uses_task_timing",
                "requires",
                "performs",
                "guarantees",
                "records_trace"
            ]
        ),
    )
}

fn sorted_node_names(core: &AilCore, kind: &str) -> Vec<String> {
    let mut names = core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == kind)
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn sorted_action_names(core: &AilCore) -> Vec<String> {
    let mut names = core
        .graph
        .nodes
        .iter()
        .filter(|node| {
            node.kind == "Action"
                && node
                    .attributes
                    .get("kind")
                    .is_none_or(|kind| kind != "CompilerPass")
        })
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn edge_target_names(core: &AilCore, source_id: &str, kind: &str) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut names = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == kind && edge.source == source_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn render_flow_edge_refs(core: &AilCore, source: &Node, kinds: &[&str]) -> String {
    let node_by_id = graph_node_by_id(core);
    let source_label = core_node_label(source);
    let mut refs = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.source == source.id && kinds.contains(&edge.kind.as_str()))
        .filter_map(|edge| {
            node_by_id
                .get(&edge.target)
                .map(|target| render_flow_edge_ref(edge, &source_label, target))
        })
        .collect::<Vec<_>>();
    refs.sort();
    format!("[{}]", refs.join(","))
}

fn render_flow_edge_ref(edge: &Edge, source_label: &str, target: &Node) -> String {
    format!(
        "{{\"kind\":{},\"source\":{},\"target\":{},\"targetName\":{},\"attributes\":{}}}",
        json_string(&edge.kind),
        json_string(source_label),
        json_string(&core_node_label(target)),
        json_string(&target.name),
        render_json_string_map(&edge.attributes)
    )
}

fn render_json_string_map(values: &BTreeMap<String, String>) -> String {
    format!(
        "{{{}}}",
        values
            .iter()
            .map(|(key, value)| format!("{}:{}", json_string(key), json_string(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn system_component_layout_summaries(core: &AilCore, component_id: &str) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut summaries = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_layout" && edge.source == component_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .map(|layout| {
            layout
                .type_name
                .as_ref()
                .map(|type_name| format!("{}: {}", layout.name, type_name))
                .unwrap_or_else(|| layout.name.clone())
        })
        .collect::<Vec<_>>();
    summaries.sort();
    summaries
}

fn system_component_allocation_summaries(core: &AilCore, component_id: &str) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut summaries = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_allocation" && edge.source == component_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .map(|allocation| {
            allocation
                .type_name
                .as_ref()
                .map(|type_name| format!("{}: {}", allocation.name, type_name))
                .unwrap_or_else(|| allocation.name.clone())
        })
        .collect::<Vec<_>>();
    summaries.sort();
    summaries
}

fn system_component_lock_guard_summaries(core: &AilCore, component_id: &str) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut summaries = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_lock_guard" && edge.source == component_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .map(|guard| {
            guard
                .type_name
                .as_ref()
                .map(|type_name| format!("{}: {}", guard.name, type_name))
                .unwrap_or_else(|| guard.name.clone())
        })
        .collect::<Vec<_>>();
    summaries.sort();
    summaries
}

fn system_component_interrupt_priority_summaries(
    core: &AilCore,
    component_id: &str,
) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut summaries = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_interrupt_priority" && edge.source == component_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .map(|priority| {
            priority
                .type_name
                .as_ref()
                .map(|type_name| format!("{}: {}", priority.name, type_name))
                .unwrap_or_else(|| priority.name.clone())
        })
        .collect::<Vec<_>>();
    summaries.sort();
    summaries
}

fn system_component_interrupt_mask_summaries(core: &AilCore, component_id: &str) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut summaries = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_interrupt_mask" && edge.source == component_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .map(|mask| {
            mask.type_name
                .as_ref()
                .map(|type_name| format!("{}: {}", mask.name, type_name))
                .unwrap_or_else(|| mask.name.clone())
        })
        .collect::<Vec<_>>();
    summaries.sort();
    summaries
}

fn system_component_scheduler_task_summaries(core: &AilCore, component_id: &str) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut summaries = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "schedules_task" && edge.source == component_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .map(|task| {
            task.type_name
                .as_ref()
                .map(|type_name| format!("{}: {}", task.name, type_name))
                .unwrap_or_else(|| task.name.clone())
        })
        .collect::<Vec<_>>();
    summaries.sort();
    summaries
}

fn system_component_scheduler_task_priority_summaries(
    core: &AilCore,
    component_id: &str,
) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut summaries = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_task_priority" && edge.source == component_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .map(|priority| {
            priority
                .type_name
                .as_ref()
                .map(|type_name| format!("{}: {}", priority.name, type_name))
                .unwrap_or_else(|| priority.name.clone())
        })
        .collect::<Vec<_>>();
    summaries.sort();
    summaries
}

fn system_component_scheduler_task_timing_summaries(
    core: &AilCore,
    component_id: &str,
) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut summaries = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_task_timing" && edge.source == component_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .map(|timing| {
            timing
                .type_name
                .as_ref()
                .map(|type_name| format!("{}: {}", timing.name, type_name))
                .unwrap_or_else(|| timing.name.clone())
        })
        .collect::<Vec<_>>();
    summaries.sort();
    summaries
}

fn edge_target_kind_names(
    core: &AilCore,
    source_id: &str,
    edge_kind: &str,
    target_kind: &str,
) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut names = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == edge_kind && edge.source == source_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .filter(|node| node.kind == target_kind)
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn render_json_array(values: Vec<String>) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| json_string(value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub fn render_ail_spec(document: &AilDocument) -> String {
    let mut lines = Vec::new();
    if !document.application.name.is_empty() {
        lines.push(format!(
            "The application {} manages {}.",
            document.application.name, document.application.purpose
        ));
        lines.push(String::new());
    }
    if !document.application.users.is_empty() {
        lines.push("The application has these users:".to_string());
        lines.push(String::new());
        for user in &document.application.users {
            lines.push(format!("- {user}"));
        }
        lines.push(String::new());
    }
    for thing in document.things.values() {
        lines.push(format!("A {} has:", thing.name));
        lines.push(String::new());
        for field in thing.fields.values() {
            lines.push(format!("- {}: {}", field.name, field.type_name));
        }
        lines.push(String::new());
    }
    if !document.application.views.is_empty() {
        lines.push("The application shows:".to_string());
        lines.push(String::new());
        for view in &document.application.views {
            lines.push(format!("- {view}"));
        }
        lines.push(String::new());
    }
    for tool in document.tools.values() {
        lines.push(format!("Tool: {}.", tool.label));
        lines.push(String::new());
        if !tool.requirements.is_empty() {
            lines.push(format!("The AI Agent may request {} when:", tool.label));
            lines.push(String::new());
            for requirement in &tool.requirements {
                lines.push(format!("- {requirement}"));
            }
            lines.push(String::new());
        }
        if !tool.inputs.is_empty() {
            lines.push("The tool needs:".to_string());
            lines.push(String::new());
            for input in tool.inputs.values() {
                lines.push(format!("- {}: {}", input.name, input.type_name));
            }
            lines.push(String::new());
        }
        if !tool.outputs.is_empty() {
            lines.push("The tool produces:".to_string());
            lines.push(String::new());
            for output in tool.outputs.values() {
                lines.push(format!("- {}: {}", output.name, output.type_name));
            }
            lines.push(String::new());
        }
        if !(tool.reads.is_empty() && tool.writes.is_empty() && tool.calls.is_empty()) {
            lines.push("The tool can:".to_string());
            lines.push(String::new());
            for read in &tool.reads {
                lines.push(format!("- read {read}"));
            }
            for call in &tool.calls {
                lines.push(format!("- call {call}"));
            }
            for write in &tool.writes {
                lines.push(format!("- write {write}"));
            }
            lines.push(String::new());
        }
        if !tool.secret_protections.is_empty() {
            lines.push("The tool must not:".to_string());
            lines.push(String::new());
            for protection in &tool.secret_protections {
                lines.push(format!("- reveal {protection}"));
            }
            lines.push(String::new());
        }
        if !tool.permissions.is_empty() {
            lines.push("The tool requires permission:".to_string());
            lines.push(String::new());
            for permission in &tool.permissions {
                lines.push(format!("- {permission}"));
            }
            lines.push(String::new());
        }
        if !tool.approvals.is_empty() {
            lines.push("The tool requires approval:".to_string());
            lines.push(String::new());
            for approval in &tool.approvals {
                lines.push(format!("- {approval}"));
            }
            lines.push(String::new());
        }
        if !tool.traces.is_empty() {
            lines.push("The tool records:".to_string());
            lines.push(String::new());
            for trace in &tool.traces {
                lines.push(format!("- {trace}"));
            }
            lines.push(String::new());
        }
        if !tool.guarantees.is_empty() {
            lines.push("The tool guarantees:".to_string());
            lines.push(String::new());
            for guarantee in &tool.guarantees {
                lines.push(format!("- {guarantee}"));
            }
            lines.push(String::new());
        }
    }
    for pass in document.compiler_passes.values() {
        lines.push(format!("Compiler pass: {}.", pass.label));
        lines.push(String::new());
        if !pass.purpose.is_empty() {
            lines.push(pass.purpose.clone());
            lines.push(String::new());
        }
        if !pass.inputs.is_empty() {
            lines.push("The pass needs:".to_string());
            lines.push(String::new());
            for input in pass.inputs.values() {
                lines.push(format!("- {}: {}", input.name, input.type_name));
            }
            lines.push(String::new());
        }
        if !pass.outputs.is_empty() {
            lines.push("The pass produces:".to_string());
            lines.push(String::new());
            for output in pass.outputs.values() {
                lines.push(format!("- {}: {}", output.name, output.type_name));
            }
            lines.push(String::new());
        }
        if !(pass.reads.is_empty()
            && pass.writes.is_empty()
            && pass.steps.is_empty()
            && pass.guarantees.is_empty()
            && pass.traces.is_empty())
        {
            lines.push(format!("When the compiler runs {}:", pass.label));
            lines.push(String::new());
            for read in &pass.reads {
                lines.push(format!("- the system reads {read}"));
            }
            for step in &pass.steps {
                lines.push(format!("- the system {step}"));
            }
            for write in &pass.writes {
                lines.push(format!("- the system adds {write}"));
            }
            for guarantee in &pass.guarantees {
                lines.push(format!("- the system guarantees {guarantee}"));
            }
            for trace in &pass.traces {
                lines.push(format!("- the system records a trace event named {trace}"));
            }
            lines.push(String::new());
        }
    }
    for component in document.system_components.values() {
        lines.push(format!("System component: {}.", component.label));
        lines.push(String::new());
        if !component.resources.is_empty() {
            lines.push("The component uses:".to_string());
            lines.push(String::new());
            for resource in component.resources.values() {
                lines.push(format!("- {}: {}", resource.name, resource.type_name));
            }
            lines.push(String::new());
        }
        if !component.owned_resources.is_empty() {
            lines.push("The component owns:".to_string());
            lines.push(String::new());
            for owned_resource in &component.owned_resources {
                lines.push(format!("- {owned_resource}"));
            }
            lines.push(String::new());
        }
        if !component.borrowed_resources.is_empty() {
            lines.push("The component borrows:".to_string());
            lines.push(String::new());
            for borrowed_resource in &component.borrowed_resources {
                lines.push(format!("- {borrowed_resource}"));
            }
            lines.push(String::new());
        }
        if !component.mutably_borrowed_resources.is_empty() {
            lines.push("The component mutably borrows:".to_string());
            lines.push(String::new());
            for borrowed_resource in &component.mutably_borrowed_resources {
                lines.push(format!("- {borrowed_resource}"));
            }
            lines.push(String::new());
        }
        if !component.resource_regions.is_empty() {
            lines.push("The component places:".to_string());
            lines.push(String::new());
            for placement in &component.resource_regions {
                lines.push(format!(
                    "- {} in {}",
                    placement.resource_name, placement.region_name
                ));
            }
            lines.push(String::new());
        }
        if !component.resource_layouts.is_empty() {
            lines.push("The component lays out:".to_string());
            lines.push(String::new());
            for layout in &component.resource_layouts {
                lines.push(format!("- {}: {}", layout.resource_name, layout.layout));
            }
            lines.push(String::new());
        }
        if !component.resource_allocations.is_empty() {
            lines.push("The component allocates:".to_string());
            lines.push(String::new());
            for allocation in &component.resource_allocations {
                lines.push(format!(
                    "- {}: {}",
                    allocation.resource_name, allocation.placement
                ));
            }
            lines.push(String::new());
        }
        if !component.lock_guards.is_empty() {
            lines.push("The component guards:".to_string());
            lines.push(String::new());
            for guard in &component.lock_guards {
                lines.push(format!(
                    "- {} with {}",
                    guard.resource_name, guard.lock_name
                ));
            }
            lines.push(String::new());
        }
        if !component.execution_contexts.is_empty() {
            lines.push("The component runs in context:".to_string());
            lines.push(String::new());
            for context in &component.execution_contexts {
                lines.push(format!("- {}", context.name));
            }
            lines.push(String::new());
        }
        if !component.interrupt_priorities.is_empty() {
            lines.push("The component sets interrupt priority:".to_string());
            lines.push(String::new());
            for priority in &component.interrupt_priorities {
                lines.push(format!(
                    "- {}: {}",
                    priority.context_name, priority.priority
                ));
            }
            lines.push(String::new());
        }
        if !component.interrupt_masks.is_empty() {
            lines.push("The component masks interrupt:".to_string());
            lines.push(String::new());
            for mask in &component.interrupt_masks {
                lines.push(format!("- {}: {}", mask.context_name, mask.mask));
            }
            lines.push(String::new());
        }
        if !component.scheduler_tasks.is_empty() {
            lines.push("The component schedules task:".to_string());
            lines.push(String::new());
            for task in &component.scheduler_tasks {
                lines.push(format!("- {}: {}", task.task_name, task.context_name));
            }
            lines.push(String::new());
        }
        if !component.scheduler_task_priorities.is_empty() {
            lines.push("The component sets task priority:".to_string());
            lines.push(String::new());
            for priority in &component.scheduler_task_priorities {
                lines.push(format!("- {}: {}", priority.task_name, priority.priority));
            }
            lines.push(String::new());
        }
        if !component.scheduler_task_timings.is_empty() {
            lines.push("The component sets task timing:".to_string());
            lines.push(String::new());
            for timing in &component.scheduler_task_timings {
                lines.push(format!(
                    "- {}: deadline {}, budget {}",
                    timing.task_name, timing.deadline, timing.budget
                ));
            }
            lines.push(String::new());
        }
        if !component.capabilities.is_empty() {
            lines.push("The component requires capability:".to_string());
            lines.push(String::new());
            for capability in &component.capabilities {
                lines.push(format!("- {capability}"));
            }
            lines.push(String::new());
        }
        if !component.effects.is_empty() {
            lines.push("The component performs:".to_string());
            lines.push(String::new());
            for effect in &component.effects {
                lines.push(format!("- {effect}"));
            }
            lines.push(String::new());
        }
        if !component.traces.is_empty() {
            lines.push("The component records:".to_string());
            lines.push(String::new());
            for trace in &component.traces {
                lines.push(format!("- {trace}"));
            }
            lines.push(String::new());
        }
        if !component.guarantees.is_empty() {
            lines.push("The component guarantees:".to_string());
            lines.push(String::new());
            for guarantee in &component.guarantees {
                lines.push(format!("- {guarantee}"));
            }
            lines.push(String::new());
        }
    }
    for action in document.actions.values() {
        lines.push(format!("Action: {}.", action.label));
        lines.push(String::new());
        lines.push(format!("When {}:", action.trigger));
        lines.push(String::new());
        for requirement in &action.requirements {
            lines.push(format!("- the system requires {requirement}"));
        }
        for read in &action.reads {
            lines.push(format!("- the system reads {read}"));
        }
        for write in &action.writes {
            lines.push(format!("- the system changes {write}"));
        }
        for protection in &action.secret_protections {
            lines.push(format!("- the system does not reveal {protection}"));
        }
        for guarantee in &action.guarantees {
            lines.push(format!("- the system guarantees {guarantee}"));
        }
        for trace in &action.traces {
            lines.push(format!("- the system records a trace event named {trace}"));
        }
        lines.push(String::new());
    }
    for failure in document.failures.values() {
        lines.push(format!(
            "Failure {} happens when {}:",
            failure.name, failure.condition
        ));
        lines.push(String::new());
        for handling in &failure.handling {
            lines.push(format!("- {handling}"));
        }
        for trace in &failure.traces {
            lines.push(format!("- the trace records {trace}"));
        }
        lines.push(String::new());
    }
    trim_trailing_blank_lines(&mut lines);
    lines.join("\n")
}

pub fn render_ail_spec_from_core(core: &AilCore) -> String {
    let document = ail_document_from_core(core);
    render_ail_spec(&document)
}

pub fn run_ail_action(
    document: &AilDocument,
    action_name: &str,
    runtime_state: BTreeMap<String, String>,
) -> Result<AilRunResult, String> {
    let action = document
        .actions
        .get(action_name)
        .ok_or_else(|| format!("unknown AIL action '{action_name}'"))?;
    let mut final_state = runtime_state;
    let mut trace = vec![format!("action {action_name} started")];

    for requirement in &action.requirements {
        let mut handled = false;
        if let Some(subject) = existence_requirement_reference(requirement) {
            let key = existence_requirement_runtime_key(document, &subject);
            if !final_state.contains_key(&key) {
                return Ok(failed_run(document, final_state, trace, "NotFound"));
            }
            handled = true;
        }
        if let Some((key, allowed_values)) = has_role_requirement(document, requirement) {
            if !final_state
                .get(&key)
                .is_some_and(|value| allowed_values.iter().any(|allowed| value == allowed))
            {
                return Ok(failed_run(
                    document,
                    final_state,
                    trace,
                    "RequirementFailed",
                ));
            }
            handled = true;
        }
        if let Some((key, allowed_values)) = has_permission_requirement(requirement) {
            if !final_state
                .get(&key)
                .is_some_and(|value| allowed_values.iter().any(|allowed| value == allowed))
            {
                return Ok(failed_run(
                    document,
                    final_state,
                    trace,
                    "RequirementFailed",
                ));
            }
            handled = true;
        }
        if let Some(keys) = input_requirement_keys(document, requirement) {
            if keys.iter().any(|key| !final_state.contains_key(key)) {
                return Ok(failed_run(
                    document,
                    final_state,
                    trace,
                    "RequirementFailed",
                ));
            }
            handled = true;
        }
        if let Some((source, key)) = field_after_requirement(document, requirement) {
            if final_state
                .get(&source)
                .zip(final_state.get(&key))
                .is_none_or(|(left, right)| left <= right)
            {
                return Ok(failed_run(
                    document,
                    final_state,
                    trace,
                    "RequirementFailed",
                ));
            }
            handled = true;
        }
        if let Some((key, forbidden)) = negative_field_requirement(document, requirement) {
            if final_state
                .get(&key)
                .is_some_and(|value| value == forbidden.as_str())
            {
                return Ok(failed_run(
                    document,
                    final_state,
                    trace,
                    "RequirementFailed",
                ));
            }
            handled = true;
        }
        if let Some((key, allowed_values)) = positive_field_requirement(document, requirement) {
            if !final_state
                .get(&key)
                .is_some_and(|value| allowed_values.iter().any(|allowed| value == allowed))
            {
                let failure_name = failed_requirement_name(document, requirement, &key);
                return Ok(failed_run(document, final_state, trace, &failure_name));
            }
            handled = true;
        }
        if handled {
            trace.push(format!("rule passed: {requirement}"));
            continue;
        }
        trace.push(format!("rule observed: {requirement}"));
    }

    for read in &action.reads {
        if let Some(key) = referenced_runtime_field_key(document, read) {
            trace.push(format!("read {key}"));
        } else {
            trace.push(format!("read {read}"));
        }
    }

    for write in &action.writes {
        if let Some((key, value)) = field_write_assignment(document, write) {
            final_state.insert(key.clone(), value.clone());
            trace.push(format!("write {key}={value}"));
        } else if let Some((source, key)) = field_copy_assignment(document, write) {
            if let Some(value) = final_state.get(&source).cloned() {
                final_state.insert(key.clone(), value);
            }
            trace.push(format!("write {key}"));
        } else if let Some(key) = referenced_runtime_field_key(document, write) {
            trace.push(format!("write {key}"));
        } else {
            trace.push(format!("effect {write}"));
        }
    }
    for guarantee in &action.guarantees {
        trace.push(format!("guarantee checked: {guarantee}"));
    }
    for event in &action.traces {
        trace.push(format!("trace {event}"));
    }
    Ok(AilRunResult {
        status: "succeeded".to_string(),
        failure: None,
        final_state,
        trace,
    })
}

pub fn render_ail_runtime_state_lines(
    document: &AilDocument,
    runtime_state: &BTreeMap<String, String>,
) -> Vec<String> {
    runtime_state
        .iter()
        .map(|(key, value)| {
            let value = if is_secret_runtime_state_key(document, key) {
                "<secret>"
            } else {
                value
            };
            format!("{key}={value}")
        })
        .collect()
}

pub fn compile_ail_bytecode(
    package: &AilPackage,
    document: &AilDocument,
) -> Result<AilBytecodeProgram, String> {
    let core = elaborate_ail_core(package, document);
    compile_ail_core_bytecode(&core)
}

pub fn compile_ail_core_bytecode(core: &AilCore) -> Result<AilBytecodeProgram, String> {
    let diagnostics = check_ail_core(core);
    if !diagnostics.is_empty() {
        return Err(format!(
            "cannot compile unchecked AIL-Core:\n{}",
            diagnostics.join("\n")
        ));
    }
    let package = AilPackage {
        metadata: core.package.clone(),
        root: PathBuf::new(),
        spec_path: PathBuf::new(),
        spec_text: String::new(),
        imports: Vec::new(),
    };
    let document = ail_document_from_core(core);
    compile_ail_document_bytecode(&package, &document)
}

pub fn compile_ail_core_native_elf(
    core: &AilCore,
    action_name: &str,
    target: &str,
) -> Result<Vec<u8>, String> {
    if target != "linux-x86_64-elf" {
        return Err(format!(
            "unsupported native target '{target}'; expected linux-x86_64-elf"
        ));
    }
    let program = compile_ail_core_bytecode(core)?;
    let diagnostics = verify_ail_bytecode(&program);
    if !diagnostics.is_empty() {
        return Err(format!(
            "cannot emit native executable from invalid AIL VM IR:\n{}",
            diagnostics.join("\n")
        ));
    }
    let action = program
        .actions
        .get(action_name)
        .ok_or_else(|| format!("unknown AIL action '{action_name}'"))?;
    emit_linux_x86_64_elf_for_action(action, &program.failures)
}

pub fn compile_ail_bytecode_native_elf(
    program: &AilBytecodeProgram,
    action_name: &str,
    target: &str,
) -> Result<Vec<u8>, String> {
    if target != "linux-x86_64-elf" {
        return Err(format!(
            "unsupported native target '{target}'; expected linux-x86_64-elf"
        ));
    }
    let diagnostics = verify_ail_bytecode(program);
    if !diagnostics.is_empty() {
        return Err(format!(
            "cannot emit native executable from invalid AIL VM IR:\n{}",
            diagnostics.join("\n")
        ));
    }
    let action = program
        .actions
        .get(action_name)
        .ok_or_else(|| format!("unknown AIL action '{action_name}'"))?;
    emit_linux_x86_64_elf_for_action(action, &program.failures)
}

struct NativeFailureBranch {
    label: String,
    trace_lines: Vec<String>,
}

enum NativeStateWrite {
    Static(String),
    Copy {
        source_prefix: String,
        destination_prefix: String,
    },
}

fn emit_linux_x86_64_elf_for_action(
    action: &AilBytecodeAction,
    failures: &BTreeMap<String, AilBytecodeFailure>,
) -> Result<Vec<u8>, String> {
    let mut exists_prefixes = Vec::new();
    let mut forbidden_exact_args = Vec::new();
    let mut required_any_exact_args = Vec::new();
    let mut required_after_args = Vec::new();
    let mut failure_branches = Vec::new();
    let mut state_writes = Vec::new();
    let mut success_trace_lines = Vec::new();
    for instruction in &action.instructions {
        match instruction.opcode.as_str() {
            "ACTION_BEGIN" => {
                if let Some(action) = instruction.operands.get("action") {
                    success_trace_lines.push(format!("action {action} started\n"));
                }
            }
            "REQUIRE_EXISTS" => {
                if let Some(key) = instruction.operands.get("key") {
                    let fail_label = format!("fail_requirement_{}", failure_branches.len());
                    exists_prefixes.push((format!("{key}="), fail_label.clone()));
                    failure_branches.push(NativeFailureBranch {
                        label: fail_label,
                        trace_lines: native_failure_trace_lines(
                            &success_trace_lines,
                            instruction.operands.get("failure"),
                            failures,
                        ),
                    });
                }
                if let Some(rule) = instruction.operands.get("rule") {
                    success_trace_lines.push(format!("rule passed: {rule}\n"));
                }
            }
            "REQUIRE_FIELD_NOT_EQUALS" => {
                if let (Some(key), Some(value)) = (
                    instruction.operands.get("key"),
                    instruction.operands.get("value"),
                ) {
                    let fail_label = format!("fail_requirement_{}", failure_branches.len());
                    forbidden_exact_args.push((format!("{key}={value}"), fail_label.clone()));
                    failure_branches.push(NativeFailureBranch {
                        label: fail_label,
                        trace_lines: native_failure_trace_lines(
                            &success_trace_lines,
                            instruction.operands.get("failure"),
                            failures,
                        ),
                    });
                }
                if let Some(rule) = instruction.operands.get("rule") {
                    success_trace_lines.push(format!("rule passed: {rule}\n"));
                }
            }
            "REQUIRE_FIELD_IN" => {
                if let (Some(key), Some(values)) = (
                    instruction.operands.get("key"),
                    instruction.operands.get("values"),
                ) {
                    let allowed_values = decode_ail_bytecode_list(values);
                    let allowed_args = allowed_values
                        .iter()
                        .map(|value| format!("{key}={value}"))
                        .collect::<Vec<_>>();
                    if !allowed_args.is_empty() {
                        let fail_label = format!("fail_requirement_{}", failure_branches.len());
                        required_any_exact_args.push((allowed_args, fail_label.clone()));
                        failure_branches.push(NativeFailureBranch {
                            label: fail_label,
                            trace_lines: native_failure_trace_lines(
                                &success_trace_lines,
                                instruction.operands.get("failure"),
                                failures,
                            ),
                        });
                    }
                }
                if let Some(rule) = instruction.operands.get("rule") {
                    success_trace_lines.push(format!("rule passed: {rule}\n"));
                }
            }
            "REQUIRE_FIELD_AFTER" => {
                if let (Some(source), Some(key)) = (
                    instruction.operands.get("source"),
                    instruction.operands.get("key"),
                ) {
                    let fail_label = format!("fail_requirement_{}", failure_branches.len());
                    required_after_args.push((
                        format!("{source}="),
                        format!("{key}="),
                        fail_label.clone(),
                    ));
                    failure_branches.push(NativeFailureBranch {
                        label: fail_label,
                        trace_lines: native_failure_trace_lines(
                            &success_trace_lines,
                            instruction.operands.get("failure"),
                            failures,
                        ),
                    });
                }
                if let Some(rule) = instruction.operands.get("rule") {
                    success_trace_lines.push(format!("rule passed: {rule}\n"));
                }
            }
            "OBSERVE_RULE" => {
                let rule = instruction
                    .operands
                    .get("rule")
                    .map(String::as_str)
                    .unwrap_or("<missing rule>");
                return Err(format!(
                    "unsupported native linux-x86_64-elf observed rule '{rule}' in action '{}'",
                    action.name
                ));
            }
            "READ_FIELD" => {
                if let Some(key) = instruction.operands.get("key") {
                    success_trace_lines.push(format!("read {key}\n"));
                }
            }
            "READ_EFFECT" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("read {text}\n"));
                }
            }
            "SET_FIELD" => {
                if let (Some(key), Some(value)) = (
                    instruction.operands.get("key"),
                    instruction.operands.get("value"),
                ) {
                    state_writes.push(NativeStateWrite::Static(format!("{key}={value}\n")));
                    success_trace_lines.push(format!("write {key}={value}\n"));
                }
            }
            "COPY_FIELD" => {
                if let (Some(source), Some(key)) = (
                    instruction.operands.get("source"),
                    instruction.operands.get("key"),
                ) {
                    state_writes.push(NativeStateWrite::Copy {
                        source_prefix: format!("{source}="),
                        destination_prefix: format!("{key}="),
                    });
                    success_trace_lines.push(format!("write {key}\n"));
                }
            }
            "WRITE_FIELD" => {
                if let Some(key) = instruction.operands.get("key") {
                    state_writes.push(NativeStateWrite::Copy {
                        source_prefix: format!("{key}.id="),
                        destination_prefix: format!("{key}.id="),
                    });
                    success_trace_lines.push(format!("write {key}\n"));
                }
            }
            "EFFECT" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("effect {text}\n"));
                }
            }
            "ASSERT_GUARANTEE" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("guarantee checked: {text}\n"));
                }
            }
            "TOOL_BEGIN" => {
                if let Some(label) = instruction.operands.get("label") {
                    success_trace_lines.push(format!("tool {label} started\n"));
                }
            }
            "TOOL_REQUIREMENT" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("tool requirement {text}\n"));
                }
            }
            "TOOL_INPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    success_trace_lines.push(format!("tool input {name}:{type_name}\n"));
                }
            }
            "TOOL_OUTPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    success_trace_lines.push(format!("tool output {name}:{type_name}\n"));
                }
            }
            "TOOL_READ" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("tool read {text}\n"));
                }
            }
            "TOOL_CALL" => {
                if let Some(target) = instruction.operands.get("target") {
                    success_trace_lines.push(format!("tool call {target}\n"));
                }
            }
            "TOOL_WRITE" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("tool write {text}\n"));
                }
            }
            "TOOL_PERMISSION" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("tool permission {text}\n"));
                }
            }
            "TOOL_APPROVAL" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("tool approval {text}\n"));
                }
            }
            "TOOL_SECRET_PROTECTION" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("tool secret protection {text}\n"));
                }
            }
            "SYSTEM_BEGIN" => {
                if let Some(label) = instruction.operands.get("label") {
                    success_trace_lines.push(format!("system component {label} started\n"));
                }
            }
            "SYSTEM_RESOURCE" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    success_trace_lines.push(format!("system resource {name}:{type_name}\n"));
                }
            }
            "SYSTEM_OWNS" => {
                if let Some(resource) = instruction.operands.get("resource") {
                    success_trace_lines.push(format!("system owns {resource}\n"));
                }
            }
            "SYSTEM_BORROWS" => {
                if let Some(resource) = instruction.operands.get("resource") {
                    success_trace_lines.push(format!("system borrows {resource}\n"));
                }
            }
            "SYSTEM_MUTABLY_BORROWS" => {
                if let Some(resource) = instruction.operands.get("resource") {
                    success_trace_lines.push(format!("system mutably borrows {resource}\n"));
                }
            }
            "SYSTEM_REGION" => {
                if let (Some(resource), Some(region)) = (
                    instruction.operands.get("resource"),
                    instruction.operands.get("region"),
                ) {
                    success_trace_lines.push(format!("system places {resource} in {region}\n"));
                }
            }
            "SYSTEM_LAYOUT" => {
                if let (Some(resource), Some(layout)) = (
                    instruction.operands.get("resource"),
                    instruction.operands.get("layout"),
                ) {
                    success_trace_lines.push(format!("system layout {resource} {layout}\n"));
                }
            }
            "SYSTEM_ALLOCATION" => {
                if let (Some(resource), Some(placement)) = (
                    instruction.operands.get("resource"),
                    instruction.operands.get("placement"),
                ) {
                    success_trace_lines.push(format!("system allocation {resource} {placement}\n"));
                }
            }
            "SYSTEM_LOCK_GUARD" => {
                if let (Some(resource), Some(lock)) = (
                    instruction.operands.get("resource"),
                    instruction.operands.get("lock"),
                ) {
                    success_trace_lines.push(format!("system lock guard {resource} with {lock}\n"));
                }
            }
            "SYSTEM_CONTEXT" => {
                if let Some(name) = instruction.operands.get("name") {
                    success_trace_lines.push(format!("system context {name}\n"));
                }
            }
            "SYSTEM_INTERRUPT_PRIORITY" => {
                if let (Some(context), Some(priority)) = (
                    instruction.operands.get("context"),
                    instruction.operands.get("priority"),
                ) {
                    success_trace_lines
                        .push(format!("system interrupt priority {context} {priority}\n"));
                }
            }
            "SYSTEM_INTERRUPT_MASK" => {
                if let (Some(context), Some(mask)) = (
                    instruction.operands.get("context"),
                    instruction.operands.get("mask"),
                ) {
                    success_trace_lines.push(format!("system interrupt mask {context} {mask}\n"));
                }
            }
            "SYSTEM_SCHEDULER_TASK" => {
                if let (Some(task), Some(context)) = (
                    instruction.operands.get("task"),
                    instruction.operands.get("context"),
                ) {
                    success_trace_lines
                        .push(format!("system scheduler task {task} in {context}\n"));
                }
            }
            "SYSTEM_TASK_PRIORITY" => {
                if let (Some(task), Some(priority)) = (
                    instruction.operands.get("task"),
                    instruction.operands.get("priority"),
                ) {
                    success_trace_lines.push(format!("system task priority {task} {priority}\n"));
                }
            }
            "SYSTEM_TASK_TIMING" => {
                if let (Some(task), Some(deadline), Some(budget)) = (
                    instruction.operands.get("task"),
                    instruction.operands.get("deadline"),
                    instruction.operands.get("budget"),
                ) {
                    success_trace_lines.push(format!(
                        "system task timing {task} deadline {deadline} budget {budget}\n"
                    ));
                }
            }
            "SYSTEM_CAPABILITY" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("system capability {text}\n"));
                }
            }
            "SYSTEM_EFFECT" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("system effect {text}\n"));
                }
            }
            "PASS_BEGIN" => {
                if let Some(label) = instruction.operands.get("label") {
                    success_trace_lines.push(format!("compiler pass {label} started\n"));
                }
            }
            "PASS_INPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    success_trace_lines.push(format!("input {name}:{type_name}\n"));
                }
            }
            "PASS_OUTPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    success_trace_lines.push(format!("output {name}:{type_name}\n"));
                }
            }
            "PASS_READ" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("pass read {text}\n"));
                }
            }
            "PASS_STEP" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("pass step {text}\n"));
                }
            }
            "PASS_WRITE" => {
                if let Some(text) = instruction.operands.get("text") {
                    success_trace_lines.push(format!("pass write {text}\n"));
                }
            }
            "CORE_INFER_READ_PERMISSIONS" => {
                success_trace_lines.push("core transform infer read permissions\n".to_string());
            }
            "EMIT_TRACE" => {
                if let Some(event) = instruction.operands.get("event") {
                    success_trace_lines.push(format!("trace {event}\n"));
                }
            }
            "RETURN_SUCCESS" => {}
            opcode => {
                return Err(format!(
                    "unsupported native linux-x86_64-elf opcode '{opcode}' in action '{}'",
                    action.name
                ));
            }
        }
    }

    let mut code = X64Code::default();
    code.emit(&[
        0x48, 0x8b, 0x1c, 0x24, // mov rbx, [rsp]
        0x4c, 0x8d, 0x64, 0x24, 0x10, // lea r12, [rsp+16]
        0x49, 0x89, 0xdd, // mov r13, rbx
        0x49, 0xff, 0xcd, // dec r13
    ]);
    for (index, (prefix, fail_label)) in exists_prefixes.iter().enumerate() {
        let label = format!("exists_prefix_{index}");
        code.emit_lea_rsi_label(&label);
        code.emit_mov_edx_imm32(prefix.len() as u32);
        code.emit_call_label("has_prefix");
        code.emit(&[0x85, 0xc0]); // test eax, eax
        code.emit_jcc_label(&[0x0f, 0x84], fail_label); // jz fail
    }
    for (index, (exact, fail_label)) in forbidden_exact_args.iter().enumerate() {
        let label = format!("forbidden_arg_{index}");
        code.emit_lea_rsi_label(&label);
        code.emit_mov_edx_imm32(exact.len() as u32);
        code.emit_call_label("has_exact");
        code.emit(&[0x85, 0xc0]); // test eax, eax
        code.emit_jcc_label(&[0x0f, 0x85], fail_label); // jnz fail
    }
    for (group_index, (allowed_args, fail_label)) in required_any_exact_args.iter().enumerate() {
        let matched_label = format!("field_in_matched_{group_index}");
        for (value_index, exact) in allowed_args.iter().enumerate() {
            let label = format!("field_in_arg_{group_index}_{value_index}");
            code.emit_lea_rsi_label(&label);
            code.emit_mov_edx_imm32(exact.len() as u32);
            code.emit_call_label("has_exact");
            code.emit(&[0x85, 0xc0]); // test eax, eax
            code.emit_jcc_label(&[0x0f, 0x85], &matched_label); // jnz matched
        }
        code.emit_jmp_label(fail_label);
        code.label(matched_label)?;
    }
    for (index, (_, _, fail_label)) in required_after_args.iter().enumerate() {
        let source_label = format!("field_after_source_{index}");
        let key_label = format!("field_after_key_{index}");
        code.emit_lea_rsi_label(&source_label);
        code.emit_mov_edx_imm32(required_after_args[index].0.len() as u32);
        code.emit_call_label("find_prefix");
        code.emit_test_rax_rax();
        code.emit_jcc_label(&[0x0f, 0x84], fail_label); // jz fail
        code.emit_mov_r14_rax();
        code.emit_lea_rsi_label(&key_label);
        code.emit_mov_edx_imm32(required_after_args[index].1.len() as u32);
        code.emit_call_label("find_prefix");
        code.emit_test_rax_rax();
        code.emit_jcc_label(&[0x0f, 0x84], fail_label); // jz fail
        code.emit_mov_r15_rax();
        code.emit_mov_rdi_r14();
        code.emit_mov_rsi_r15();
        code.emit_call_label("cstring_gt");
        code.emit(&[0x85, 0xc0]); // test eax, eax
        code.emit_jcc_label(&[0x0f, 0x84], fail_label); // jz fail
    }
    for (index, write) in state_writes.iter().enumerate() {
        match write {
            NativeStateWrite::Static(line) => {
                let label = format!("state_write_{index}");
                code.emit_write_label(1, &label, line.len() as u32);
            }
            NativeStateWrite::Copy {
                source_prefix,
                destination_prefix,
            } => {
                let source_label = format!("state_copy_source_{index}");
                let destination_label = format!("state_copy_destination_{index}");
                let done_label = format!("state_copy_done_{index}");
                code.emit_lea_rsi_label(&source_label);
                code.emit_mov_edx_imm32(source_prefix.len() as u32);
                code.emit_call_label("find_prefix");
                code.emit_test_rax_rax();
                code.emit_jcc_label(&[0x0f, 0x84], &done_label); // jz done
                code.emit_mov_r14_rax();
                code.emit_write_label(1, &destination_label, destination_prefix.len() as u32);
                code.emit_mov_rdi_r14();
                code.emit_call_label("cstring_len");
                code.emit_write_r14_with_rax_len(1);
                code.emit_write_label(1, "state_copy_newline", 1);
                code.label(done_label)?;
            }
        }
    }
    for (index, line) in success_trace_lines.iter().enumerate() {
        let label = format!("trace_write_{index}");
        code.emit_write_label(2, &label, line.len() as u32);
    }
    code.label("success")?;
    code.emit_exit(0);
    for branch in &failure_branches {
        code.label(&branch.label)?;
        for (index, line) in branch.trace_lines.iter().enumerate() {
            let label = format!("{}_trace_{index}", branch.label);
            code.emit_write_label(2, &label, line.len() as u32);
        }
        code.emit_exit(1);
    }
    emit_has_prefix(&mut code)?;
    emit_has_exact(&mut code)?;
    emit_find_prefix(&mut code)?;
    emit_cstring_len(&mut code)?;
    emit_cstring_gt(&mut code)?;
    for (index, (prefix, _)) in exists_prefixes.iter().enumerate() {
        code.label(format!("exists_prefix_{index}"))?;
        code.emit(prefix.as_bytes());
    }
    for (index, (exact, _)) in forbidden_exact_args.iter().enumerate() {
        code.label(format!("forbidden_arg_{index}"))?;
        code.emit(exact.as_bytes());
    }
    for (group_index, (allowed_args, _)) in required_any_exact_args.iter().enumerate() {
        for (value_index, exact) in allowed_args.iter().enumerate() {
            code.label(format!("field_in_arg_{group_index}_{value_index}"))?;
            code.emit(exact.as_bytes());
        }
    }
    for (index, (source, key, _)) in required_after_args.iter().enumerate() {
        code.label(format!("field_after_source_{index}"))?;
        code.emit(source.as_bytes());
        code.label(format!("field_after_key_{index}"))?;
        code.emit(key.as_bytes());
    }
    let has_state_copies = state_writes
        .iter()
        .any(|write| matches!(write, NativeStateWrite::Copy { .. }));
    for (index, write) in state_writes.iter().enumerate() {
        match write {
            NativeStateWrite::Static(line) => {
                code.label(format!("state_write_{index}"))?;
                code.emit(line.as_bytes());
            }
            NativeStateWrite::Copy {
                source_prefix,
                destination_prefix,
            } => {
                code.label(format!("state_copy_source_{index}"))?;
                code.emit(source_prefix.as_bytes());
                code.label(format!("state_copy_destination_{index}"))?;
                code.emit(destination_prefix.as_bytes());
            }
        }
    }
    if has_state_copies {
        code.label("state_copy_newline")?;
        code.emit(b"\n");
    }
    for (index, line) in success_trace_lines.iter().enumerate() {
        code.label(format!("trace_write_{index}"))?;
        code.emit(line.as_bytes());
    }
    for branch in &failure_branches {
        for (index, line) in branch.trace_lines.iter().enumerate() {
            code.label(format!("{}_trace_{index}", branch.label))?;
            code.emit(line.as_bytes());
        }
    }
    let code = code.finish()?;
    Ok(wrap_linux_x86_64_elf(&code))
}

fn native_failure_trace_lines(
    prefix: &[String],
    failure_name: Option<&String>,
    failures: &BTreeMap<String, AilBytecodeFailure>,
) -> Vec<String> {
    let failure_name = failure_name
        .map(String::as_str)
        .unwrap_or("RequirementFailed");
    let mut trace_lines = prefix.to_vec();
    trace_lines.push(format!("failure {failure_name}\n"));
    if let Some(failure) = failures.get(failure_name) {
        for event in &failure.traces {
            trace_lines.push(format!("trace {event}\n"));
        }
    }
    trace_lines
}

fn emit_has_prefix(code: &mut X64Code) -> Result<(), String> {
    code.label("has_prefix")?;
    code.emit(&[0x45, 0x31, 0xc0]); // xor r8d, r8d
    code.label("prefix_loop")?;
    code.emit(&[0x4d, 0x39, 0xe8]); // cmp r8, r13
    code.emit_jcc_label(&[0x0f, 0x83], "prefix_no"); // jae prefix_no
    code.emit(&[
        0x4b, 0x8b, 0x3c, 0xc4, // mov rdi, [r12+r8*8]
        0x31, 0xc9, // xor ecx, ecx
    ]);
    code.label("prefix_cmp")?;
    code.emit(&[0x39, 0xd1]); // cmp ecx, edx
    code.emit_jcc_label(&[0x0f, 0x83], "prefix_yes"); // jae prefix_yes
    code.emit(&[
        0x8a, 0x04, 0x0f, // mov al, [rdi+rcx]
        0x3a, 0x04, 0x0e, // cmp al, [rsi+rcx]
    ]);
    code.emit_jcc_label(&[0x0f, 0x85], "prefix_next"); // jne prefix_next
    code.emit(&[0x48, 0xff, 0xc1]); // inc rcx
    code.emit_jmp_label("prefix_cmp");
    code.label("prefix_next")?;
    code.emit(&[0x49, 0xff, 0xc0]); // inc r8
    code.emit_jmp_label("prefix_loop");
    code.label("prefix_yes")?;
    code.emit(&[0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3]); // mov eax, 1; ret
    code.label("prefix_no")?;
    code.emit(&[0x31, 0xc0, 0xc3]); // xor eax, eax; ret
    Ok(())
}

fn emit_has_exact(code: &mut X64Code) -> Result<(), String> {
    code.label("has_exact")?;
    code.emit(&[0x45, 0x31, 0xc0]); // xor r8d, r8d
    code.label("exact_loop")?;
    code.emit(&[0x4d, 0x39, 0xe8]); // cmp r8, r13
    code.emit_jcc_label(&[0x0f, 0x83], "exact_no"); // jae exact_no
    code.emit(&[
        0x4b, 0x8b, 0x3c, 0xc4, // mov rdi, [r12+r8*8]
        0x31, 0xc9, // xor ecx, ecx
    ]);
    code.label("exact_cmp")?;
    code.emit(&[0x39, 0xd1]); // cmp ecx, edx
    code.emit_jcc_label(&[0x0f, 0x83], "exact_end_check"); // jae exact_end_check
    code.emit(&[
        0x8a, 0x04, 0x0f, // mov al, [rdi+rcx]
        0x3a, 0x04, 0x0e, // cmp al, [rsi+rcx]
    ]);
    code.emit_jcc_label(&[0x0f, 0x85], "exact_next"); // jne exact_next
    code.emit(&[0x48, 0xff, 0xc1]); // inc rcx
    code.emit_jmp_label("exact_cmp");
    code.label("exact_end_check")?;
    code.emit(&[0x80, 0x3c, 0x0f, 0x00]); // cmp byte [rdi+rcx], 0
    code.emit_jcc_label(&[0x0f, 0x84], "exact_yes"); // je exact_yes
    code.label("exact_next")?;
    code.emit(&[0x49, 0xff, 0xc0]); // inc r8
    code.emit_jmp_label("exact_loop");
    code.label("exact_yes")?;
    code.emit(&[0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3]); // mov eax, 1; ret
    code.label("exact_no")?;
    code.emit(&[0x31, 0xc0, 0xc3]); // xor eax, eax; ret
    Ok(())
}

fn emit_find_prefix(code: &mut X64Code) -> Result<(), String> {
    code.label("find_prefix")?;
    code.emit(&[0x45, 0x31, 0xc0]); // xor r8d, r8d
    code.label("find_prefix_loop")?;
    code.emit(&[0x4d, 0x39, 0xe8]); // cmp r8, r13
    code.emit_jcc_label(&[0x0f, 0x83], "find_prefix_no"); // jae find_prefix_no
    code.emit(&[
        0x4b, 0x8b, 0x3c, 0xc4, // mov rdi, [r12+r8*8]
        0x31, 0xc9, // xor ecx, ecx
    ]);
    code.label("find_prefix_cmp")?;
    code.emit(&[0x39, 0xd1]); // cmp ecx, edx
    code.emit_jcc_label(&[0x0f, 0x83], "find_prefix_yes"); // jae find_prefix_yes
    code.emit(&[
        0x8a, 0x04, 0x0f, // mov al, [rdi+rcx]
        0x3a, 0x04, 0x0e, // cmp al, [rsi+rcx]
    ]);
    code.emit_jcc_label(&[0x0f, 0x85], "find_prefix_next"); // jne find_prefix_next
    code.emit(&[0x48, 0xff, 0xc1]); // inc rcx
    code.emit_jmp_label("find_prefix_cmp");
    code.label("find_prefix_next")?;
    code.emit(&[0x49, 0xff, 0xc0]); // inc r8
    code.emit_jmp_label("find_prefix_loop");
    code.label("find_prefix_yes")?;
    code.emit(&[0x48, 0x8d, 0x04, 0x17, 0xc3]); // lea rax, [rdi+rdx]; ret
    code.label("find_prefix_no")?;
    code.emit(&[0x31, 0xc0, 0xc3]); // xor eax, eax; ret
    Ok(())
}

fn emit_cstring_len(code: &mut X64Code) -> Result<(), String> {
    code.label("cstring_len")?;
    code.emit(&[0x48, 0x31, 0xc0]); // xor rax, rax
    code.label("cstring_len_loop")?;
    code.emit(&[0x80, 0x3c, 0x07, 0x00]); // cmp byte [rdi+rax], 0
    code.emit_jcc_label(&[0x0f, 0x84], "cstring_len_done"); // je cstring_len_done
    code.emit(&[0x48, 0xff, 0xc0]); // inc rax
    code.emit_jmp_label("cstring_len_loop");
    code.label("cstring_len_done")?;
    code.emit(&[0xc3]); // ret
    Ok(())
}

fn emit_cstring_gt(code: &mut X64Code) -> Result<(), String> {
    code.label("cstring_gt")?;
    code.emit(&[0x31, 0xc9]); // xor ecx, ecx
    code.label("cstring_gt_loop")?;
    code.emit(&[
        0x8a, 0x04, 0x0f, // mov al, [rdi+rcx]
        0x44, 0x8a, 0x04, 0x0e, // mov r8b, [rsi+rcx]
        0x44, 0x38, 0xc0, // cmp al, r8b
    ]);
    code.emit_jcc_label(&[0x0f, 0x87], "cstring_gt_yes"); // ja yes
    code.emit_jcc_label(&[0x0f, 0x82], "cstring_gt_no"); // jb no
    code.emit(&[0x84, 0xc0]); // test al, al
    code.emit_jcc_label(&[0x0f, 0x84], "cstring_gt_no"); // je no
    code.emit(&[0x48, 0xff, 0xc1]); // inc rcx
    code.emit_jmp_label("cstring_gt_loop");
    code.label("cstring_gt_yes")?;
    code.emit(&[0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3]); // mov eax, 1; ret
    code.label("cstring_gt_no")?;
    code.emit(&[0x31, 0xc0, 0xc3]); // xor eax, eax; ret
    Ok(())
}

#[derive(Default)]
struct X64Code {
    bytes: Vec<u8>,
    labels: BTreeMap<String, usize>,
    rel32_patches: Vec<(usize, String)>,
}

impl X64Code {
    fn emit(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn label(&mut self, name: impl Into<String>) -> Result<(), String> {
        let name = name.into();
        if self.labels.insert(name.clone(), self.bytes.len()).is_some() {
            return Err(format!("duplicate x86_64 code label '{name}'"));
        }
        Ok(())
    }

    fn emit_lea_rsi_label(&mut self, label: &str) {
        self.emit(&[0x48, 0x8d, 0x35]);
        self.emit_rel32(label);
    }

    fn emit_mov_edx_imm32(&mut self, value: u32) {
        self.emit(&[0xba]);
        self.emit(&value.to_le_bytes());
    }

    fn emit_test_rax_rax(&mut self) {
        self.emit(&[0x48, 0x85, 0xc0]);
    }

    fn emit_mov_r14_rax(&mut self) {
        self.emit(&[0x49, 0x89, 0xc6]);
    }

    fn emit_mov_r15_rax(&mut self) {
        self.emit(&[0x49, 0x89, 0xc7]);
    }

    fn emit_mov_rdi_r14(&mut self) {
        self.emit(&[0x4c, 0x89, 0xf7]);
    }

    fn emit_mov_rsi_r15(&mut self) {
        self.emit(&[0x4c, 0x89, 0xfe]);
    }

    fn emit_write_label(&mut self, fd: u32, label: &str, len: u32) {
        self.emit(&[
            0xb8, 0x01, 0x00, 0x00, 0x00, // mov eax, 1
            0xbf, // mov edi, fd
        ]);
        self.emit(&fd.to_le_bytes());
        self.emit_lea_rsi_label(label);
        self.emit_mov_edx_imm32(len);
        self.emit(&[0x0f, 0x05]); // syscall
    }

    fn emit_write_r14_with_rax_len(&mut self, fd: u32) {
        self.emit(&[
            0x48, 0x89, 0xc2, // mov rdx, rax
            0xb8, 0x01, 0x00, 0x00, 0x00, // mov eax, 1
            0xbf, // mov edi, fd
        ]);
        self.emit(&fd.to_le_bytes());
        self.emit(&[
            0x4c, 0x89, 0xf6, // mov rsi, r14
            0x0f, 0x05, // syscall
        ]);
    }

    fn emit_call_label(&mut self, label: &str) {
        self.emit(&[0xe8]);
        self.emit_rel32(label);
    }

    fn emit_jmp_label(&mut self, label: &str) {
        self.emit(&[0xe9]);
        self.emit_rel32(label);
    }

    fn emit_jcc_label(&mut self, opcode: &[u8], label: &str) {
        self.emit(opcode);
        self.emit_rel32(label);
    }

    fn emit_exit(&mut self, status: u8) {
        self.emit(&[0xb8, 0x3c, 0x00, 0x00, 0x00]);
        if status == 0 {
            self.emit(&[0x31, 0xff]);
        } else {
            self.emit(&[0xbf]);
            self.emit(&(u32::from(status)).to_le_bytes());
        }
        self.emit(&[0x0f, 0x05]);
    }

    fn emit_rel32(&mut self, label: &str) {
        let position = self.bytes.len();
        self.bytes.extend_from_slice(&[0; 4]);
        self.rel32_patches.push((position, label.to_string()));
    }

    fn finish(mut self) -> Result<Vec<u8>, String> {
        for (position, label) in &self.rel32_patches {
            let target = *self
                .labels
                .get(label)
                .ok_or_else(|| format!("unknown x86_64 code label '{label}'"))?;
            let next = position + 4;
            let offset = target as isize - next as isize;
            let offset = i32::try_from(offset)
                .map_err(|_| format!("x86_64 rel32 patch to '{label}' is out of range"))?;
            self.bytes[*position..*position + 4].copy_from_slice(&offset.to_le_bytes());
        }
        Ok(self.bytes)
    }
}

fn wrap_linux_x86_64_elf(code: &[u8]) -> Vec<u8> {
    let elf_header_size = 64u16;
    let program_header_size = 56u16;
    let code_offset = u64::from(elf_header_size + program_header_size);
    let image_base = 0x400000u64;
    let file_size = code_offset + code.len() as u64;
    let mut out = Vec::with_capacity(file_size as usize);

    out.extend_from_slice(b"\x7fELF");
    out.push(2); // ELFCLASS64
    out.push(1); // ELFDATA2LSB
    out.push(1); // EV_CURRENT
    out.push(0); // System V ABI
    out.extend_from_slice(&[0; 8]);

    push_u16_le(&mut out, 2); // ET_EXEC
    push_u16_le(&mut out, 0x3e); // EM_X86_64
    push_u32_le(&mut out, 1); // EV_CURRENT
    push_u64_le(&mut out, image_base + code_offset);
    push_u64_le(&mut out, u64::from(elf_header_size));
    push_u64_le(&mut out, 0);
    push_u32_le(&mut out, 0);
    push_u16_le(&mut out, elf_header_size);
    push_u16_le(&mut out, program_header_size);
    push_u16_le(&mut out, 1);
    push_u16_le(&mut out, 0);
    push_u16_le(&mut out, 0);
    push_u16_le(&mut out, 0);

    push_u32_le(&mut out, 1); // PT_LOAD
    push_u32_le(&mut out, 5); // PF_R | PF_X
    push_u64_le(&mut out, 0);
    push_u64_le(&mut out, image_base);
    push_u64_le(&mut out, image_base);
    push_u64_le(&mut out, file_size);
    push_u64_le(&mut out, file_size);
    push_u64_le(&mut out, 0x1000);

    debug_assert_eq!(out.len(), code_offset as usize);
    out.extend_from_slice(code);
    out
}

fn push_u16_le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64_le(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn compile_ail_document_bytecode(
    package: &AilPackage,
    document: &AilDocument,
) -> Result<AilBytecodeProgram, String> {
    let actions = match package.metadata.profile.as_str() {
        "Application" => document
            .actions
            .iter()
            .map(|(name, action)| (name.clone(), compile_ail_action_bytecode(document, action)))
            .collect(),
        "AgentTool" => document
            .tools
            .iter()
            .map(|(name, tool)| (name.clone(), compile_ail_tool_bytecode(tool)))
            .collect(),
        "Compiler" => document
            .compiler_passes
            .iter()
            .map(|(name, pass)| (name.clone(), compile_ail_compiler_pass_bytecode(pass)))
            .collect(),
        "System" => document
            .system_components
            .iter()
            .map(|(name, component)| (name.clone(), compile_ail_system_bytecode(component)))
            .collect(),
        profile => {
            return Err(format!(
                "ail-lower currently supports Application, AgentTool, Compiler, and System packages, not {profile}"
            ));
        }
    };
    let failures = document
        .failures
        .iter()
        .map(|(name, failure)| {
            (
                name.clone(),
                AilBytecodeFailure {
                    name: name.clone(),
                    traces: failure.traces.clone(),
                },
            )
        })
        .collect();
    Ok(AilBytecodeProgram {
        package_name: package.metadata.name.clone(),
        package_version: package.metadata.version.clone(),
        profile: package.metadata.profile.clone(),
        actions,
        failures,
    })
}

fn ail_document_from_core(core: &AilCore) -> AilDocument {
    let node_by_id = graph_node_by_id(core);
    let mut document = AilDocument {
        application: AilApplication::default(),
        things: BTreeMap::new(),
        tools: BTreeMap::new(),
        compiler_passes: BTreeMap::new(),
        system_components: BTreeMap::new(),
        actions: BTreeMap::new(),
        failures: BTreeMap::new(),
    };

    if let Some(application) = core
        .graph
        .nodes
        .iter()
        .find(|node| node.kind == "Application")
    {
        let application_children = outgoing_nodes(core, &node_by_id, application, "contains");
        document.application = AilApplication {
            name: application.name.clone(),
            purpose: application
                .attributes
                .get("purpose")
                .cloned()
                .unwrap_or_default(),
            users: application_children
                .iter()
                .filter(|node| node.kind == "User")
                .map(|node| node.name.clone())
                .collect(),
            views: application_children
                .into_iter()
                .filter(|node| node.kind == "View")
                .map(|node| node.name)
                .collect(),
        };
    }

    for thing_node in core.graph.nodes.iter().filter(|node| node.kind == "Thing") {
        let mut thing = AilThing {
            name: thing_node.name.clone(),
            fields: BTreeMap::new(),
            provenance: node_provenance(core, &thing_node.id).unwrap_or_default(),
        };
        for field_node in outgoing_nodes(core, &node_by_id, thing_node, "has_field")
            .into_iter()
            .filter(|node| node.kind == "Field")
        {
            let field_name = local_core_name(&field_node.name, &thing.name);
            thing.fields.insert(
                field_name.clone(),
                AilField {
                    name: field_name,
                    type_name: field_node.type_name.clone().unwrap_or_default(),
                    is_secret: field_node
                        .attributes
                        .get("secret")
                        .is_some_and(|value| value == "true"),
                    provenance: node_provenance(core, &field_node.id).unwrap_or_default(),
                },
            );
        }
        document.things.insert(thing.name.clone(), thing);
    }

    for failure_node in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Failure")
        .filter(|node| {
            node.attributes
                .get("declared")
                .is_some_and(|value| value == "true")
        })
    {
        let mut failure = AilFailure {
            name: failure_node.name.clone(),
            condition: failure_node
                .attributes
                .get("condition")
                .cloned()
                .unwrap_or_default(),
            provenance: node_provenance(core, &failure_node.id).unwrap_or_default(),
            ..AilFailure::default()
        };
        failure.handling = outgoing_nodes(core, &node_by_id, failure_node, "handles_failure")
            .into_iter()
            .filter(|node| node.kind == "Effect")
            .map(|node| {
                core_node_provenance_payload(
                    core,
                    &node,
                    &format!("failure:{}.handling:", failure.name),
                )
                .unwrap_or(node.name)
            })
            .collect();
        failure.traces = outgoing_nodes(core, &node_by_id, failure_node, "records_trace")
            .into_iter()
            .filter(|node| node.kind == "Trace")
            .map(|node| node.name)
            .collect();
        document.failures.insert(failure.name.clone(), failure);
    }

    for action_node in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Action")
        .filter(|node| {
            node.attributes
                .get("kind")
                .is_none_or(|kind| kind != "CompilerPass")
        })
    {
        let action = AilAction {
            name: action_node.name.clone(),
            label: action_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| action_node.name.clone()),
            trigger: action_node
                .attributes
                .get("trigger")
                .cloned()
                .unwrap_or_default(),
            requirements: outgoing_nodes(core, &node_by_id, action_node, "requires")
                .into_iter()
                .filter(|node| node.kind == "Rule")
                .map(|node| node.name)
                .collect(),
            reads: outgoing_edge_payloads(core, &node_by_id, action_node, "reads", "read"),
            writes: outgoing_edge_payloads(core, &node_by_id, action_node, "writes", "write"),
            failures: outgoing_nodes(core, &node_by_id, action_node, "may_fail_with")
                .into_iter()
                .filter(|node| node.kind == "Failure")
                .map(|node| node.name)
                .collect(),
            guarantees: outgoing_nodes(core, &node_by_id, action_node, "guarantees")
                .into_iter()
                .filter(|node| node.kind == "Guarantee")
                .map(|node| node.name)
                .collect(),
            traces: outgoing_nodes(core, &node_by_id, action_node, "records_trace")
                .into_iter()
                .filter(|node| node.kind == "Trace")
                .map(|node| node.name)
                .collect(),
            secret_protections: outgoing_nodes(core, &node_by_id, action_node, "protects_secret")
                .into_iter()
                .map(|node| node.name)
                .collect(),
            provenance: node_provenance(core, &action_node.id).unwrap_or_default(),
        };
        document.actions.insert(action.name.clone(), action);
    }

    for tool_node in core.graph.nodes.iter().filter(|node| node.kind == "Tool") {
        let mut tool = AilTool {
            name: tool_node.name.clone(),
            label: tool_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| tool_node.name.clone()),
            provenance: node_provenance(core, &tool_node.id).unwrap_or_default(),
            ..AilTool::default()
        };
        for input_node in outgoing_nodes(core, &node_by_id, tool_node, "has_input")
            .into_iter()
            .filter(|node| node.kind == "Input")
        {
            let input_name = local_core_name(&input_node.name, &tool.name);
            tool.inputs.insert(
                input_name.clone(),
                AilToolSlot {
                    name: input_name,
                    type_name: input_node.type_name.clone().unwrap_or_default(),
                    is_secret: input_node
                        .attributes
                        .get("secret")
                        .is_some_and(|value| value == "true"),
                    provenance: node_provenance(core, &input_node.id).unwrap_or_default(),
                },
            );
        }
        for output_node in outgoing_nodes(core, &node_by_id, tool_node, "has_output")
            .into_iter()
            .filter(|node| node.kind == "Output")
        {
            let output_name = local_core_name(&output_node.name, &tool.name);
            tool.outputs.insert(
                output_name.clone(),
                AilToolSlot {
                    name: output_name,
                    type_name: output_node.type_name.clone().unwrap_or_default(),
                    is_secret: output_node
                        .attributes
                        .get("secret")
                        .is_some_and(|value| value == "true"),
                    provenance: node_provenance(core, &output_node.id).unwrap_or_default(),
                },
            );
        }
        tool.requirements = outgoing_nodes(core, &node_by_id, tool_node, "requires")
            .into_iter()
            .filter(|node| node.kind == "Rule")
            .map(|node| node.name)
            .collect();
        tool.permissions = outgoing_nodes(core, &node_by_id, tool_node, "requires")
            .into_iter()
            .filter(|node| node.kind == "Permission")
            .map(|node| node.name)
            .collect();
        tool.approvals = outgoing_nodes(core, &node_by_id, tool_node, "requires_approval")
            .into_iter()
            .filter(|node| node.kind == "Approval")
            .map(|node| node.name)
            .collect();
        tool.reads = outgoing_edge_payloads(core, &node_by_id, tool_node, "reads", "read");
        tool.writes = outgoing_edge_payloads(core, &node_by_id, tool_node, "writes", "write");
        tool.calls = outgoing_edge_payloads(core, &node_by_id, tool_node, "calls", "call");
        tool.failures = outgoing_nodes(core, &node_by_id, tool_node, "may_fail_with")
            .into_iter()
            .filter(|node| node.kind == "Failure")
            .map(|node| node.name)
            .collect();
        tool.guarantees = outgoing_nodes(core, &node_by_id, tool_node, "guarantees")
            .into_iter()
            .filter(|node| node.kind == "Guarantee")
            .map(|node| node.name)
            .collect();
        tool.traces = outgoing_nodes(core, &node_by_id, tool_node, "records_trace")
            .into_iter()
            .filter(|node| node.kind == "Trace")
            .map(|node| node.name)
            .collect();
        tool.secret_protections = outgoing_nodes(core, &node_by_id, tool_node, "protects_secret")
            .into_iter()
            .map(|node| local_core_name(&node.name, &tool.name))
            .collect();
        document.tools.insert(tool.name.clone(), tool);
    }

    for pass_node in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Action")
        .filter(|node| {
            node.attributes
                .get("kind")
                .is_some_and(|kind| kind == "CompilerPass")
        })
    {
        let mut pass = AilCompilerPass {
            name: pass_node.name.clone(),
            label: pass_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| pass_node.name.clone()),
            purpose: pass_node
                .attributes
                .get("purpose")
                .cloned()
                .unwrap_or_default(),
            provenance: node_provenance(core, &pass_node.id).unwrap_or_default(),
            ..AilCompilerPass::default()
        };
        for value_node in outgoing_nodes(core, &node_by_id, pass_node, "reads")
            .into_iter()
            .filter(|node| node.kind == "Value")
        {
            let value_name = local_core_name(&value_node.name, &pass.name);
            pass.inputs.insert(
                value_name.clone(),
                AilPassValue {
                    name: value_name,
                    type_name: value_node.type_name.clone().unwrap_or_default(),
                    provenance: node_provenance(core, &value_node.id).unwrap_or_default(),
                },
            );
        }
        for value_node in outgoing_nodes(core, &node_by_id, pass_node, "writes")
            .into_iter()
            .filter(|node| node.kind == "Value")
        {
            let value_name = local_core_name(&value_node.name, &pass.name);
            pass.outputs.insert(
                value_name.clone(),
                AilPassValue {
                    name: value_name,
                    type_name: value_node.type_name.clone().unwrap_or_default(),
                    provenance: node_provenance(core, &value_node.id).unwrap_or_default(),
                },
            );
        }
        pass.reads = outgoing_edge_payloads(core, &node_by_id, pass_node, "reads", "read")
            .into_iter()
            .filter(|text| !pass.inputs.contains_key(text))
            .collect();
        pass.writes = outgoing_edge_payloads(core, &node_by_id, pass_node, "writes", "write")
            .into_iter()
            .filter(|text| !pass.outputs.contains_key(text))
            .collect();
        pass.steps = outgoing_nodes(core, &node_by_id, pass_node, "contains")
            .into_iter()
            .filter(|node| node.kind == "Step")
            .map(|node| node.name)
            .collect();
        pass.failures = outgoing_nodes(core, &node_by_id, pass_node, "may_fail_with")
            .into_iter()
            .filter(|node| node.kind == "Failure")
            .map(|node| node.name)
            .collect();
        pass.guarantees = outgoing_nodes(core, &node_by_id, pass_node, "guarantees")
            .into_iter()
            .filter(|node| node.kind == "Guarantee")
            .map(|node| node.name)
            .collect();
        pass.traces = outgoing_nodes(core, &node_by_id, pass_node, "records_trace")
            .into_iter()
            .filter(|node| node.kind == "Trace")
            .map(|node| node.name)
            .collect();
        document.compiler_passes.insert(pass.name.clone(), pass);
    }

    for component_node in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "SystemComponent")
    {
        let mut component = AilSystemComponent {
            name: component_node.name.clone(),
            label: component_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| component_node.name.clone()),
            provenance: node_provenance(core, &component_node.id).unwrap_or_default(),
            ..AilSystemComponent::default()
        };
        for resource_node in outgoing_nodes(core, &node_by_id, component_node, "uses_resource")
            .into_iter()
            .filter(|node| node.kind == "Resource")
        {
            let resource_name = local_core_name(&resource_node.name, &component.name);
            component.resources.insert(
                resource_name.clone(),
                AilSystemResource {
                    name: resource_name,
                    type_name: resource_node.type_name.clone().unwrap_or_default(),
                    provenance: node_provenance(core, &resource_node.id).unwrap_or_default(),
                },
            );
        }
        component.owned_resources = local_target_names(
            core,
            &node_by_id,
            component_node,
            "owns_resource",
            &component.name,
        );
        component.borrowed_resources = local_target_names(
            core,
            &node_by_id,
            component_node,
            "borrows_resource",
            &component.name,
        );
        component.mutably_borrowed_resources = local_target_names(
            core,
            &node_by_id,
            component_node,
            "mutably_borrows_resource",
            &component.name,
        );
        component.resource_regions = system_regions_from_core(core, &node_by_id, &component.name);
        component.resource_layouts = system_resource_attrs(
            core,
            &node_by_id,
            component_node,
            "uses_layout",
            &component.name,
        )
        .into_iter()
        .map(
            |(resource_name, value, provenance)| AilSystemResourceLayout {
                resource_name,
                layout: value,
                provenance,
            },
        )
        .collect();
        component.resource_allocations = system_resource_attrs(
            core,
            &node_by_id,
            component_node,
            "uses_allocation",
            &component.name,
        )
        .into_iter()
        .map(
            |(resource_name, value, provenance)| AilSystemResourceAllocation {
                resource_name,
                placement: value,
                provenance,
            },
        )
        .collect();
        component.lock_guards =
            outgoing_nodes(core, &node_by_id, component_node, "uses_lock_guard")
                .into_iter()
                .filter(|node| node.kind == "LockGuard")
                .map(|node| AilSystemLockGuard {
                    resource_name: node
                        .attributes
                        .get("resource")
                        .cloned()
                        .unwrap_or_else(|| local_core_name(&node.name, &component.name)),
                    lock_name: node.attributes.get("lock").cloned().unwrap_or_default(),
                    provenance: node_provenance(core, &node.id).unwrap_or_default(),
                })
                .collect();
        component.execution_contexts =
            outgoing_nodes(core, &node_by_id, component_node, "runs_in_context")
                .into_iter()
                .filter(|node| node.kind == "ExecutionContext")
                .map(|node| AilSystemExecutionContext {
                    name: node
                        .attributes
                        .get("context")
                        .cloned()
                        .unwrap_or_else(|| local_core_name(&node.name, &component.name)),
                    provenance: node_provenance(core, &node.id).unwrap_or_default(),
                })
                .collect();
        component.interrupt_priorities = system_context_attrs(
            core,
            &node_by_id,
            component_node,
            "uses_interrupt_priority",
            &component.name,
        )
        .into_iter()
        .map(
            |(context_name, value, provenance)| AilSystemInterruptPriority {
                context_name,
                priority: value,
                provenance,
            },
        )
        .collect();
        component.interrupt_masks = system_context_attrs(
            core,
            &node_by_id,
            component_node,
            "uses_interrupt_mask",
            &component.name,
        )
        .into_iter()
        .map(|(context_name, value, provenance)| AilSystemInterruptMask {
            context_name,
            mask: value,
            provenance,
        })
        .collect();
        component.scheduler_tasks =
            outgoing_nodes(core, &node_by_id, component_node, "schedules_task")
                .into_iter()
                .filter(|node| node.kind == "SchedulerTask")
                .map(|node| AilSystemSchedulerTask {
                    task_name: local_core_name(&node.name, &component.name),
                    context_name: node.attributes.get("context").cloned().unwrap_or_default(),
                    provenance: node_provenance(core, &node.id).unwrap_or_default(),
                })
                .collect();
        component.scheduler_task_priorities = system_task_attrs(
            core,
            &node_by_id,
            component_node,
            "uses_task_priority",
            &component.name,
        )
        .into_iter()
        .map(
            |(task_name, value, provenance)| AilSystemSchedulerTaskPriority {
                task_name,
                priority: value,
                provenance,
            },
        )
        .collect();
        component.scheduler_task_timings =
            outgoing_nodes(core, &node_by_id, component_node, "uses_task_timing")
                .into_iter()
                .filter(|node| node.kind == "SchedulerTaskTiming")
                .map(|node| AilSystemSchedulerTaskTiming {
                    task_name: node
                        .attributes
                        .get("task")
                        .cloned()
                        .unwrap_or_else(|| local_core_name(&node.name, &component.name)),
                    deadline: node.attributes.get("deadline").cloned().unwrap_or_default(),
                    budget: node.attributes.get("budget").cloned().unwrap_or_default(),
                    provenance: node_provenance(core, &node.id).unwrap_or_default(),
                })
                .collect();
        component.capabilities = outgoing_nodes(core, &node_by_id, component_node, "requires")
            .into_iter()
            .filter(|node| node.kind == "Capability")
            .map(|node| node.name)
            .collect();
        component.effects = outgoing_nodes(core, &node_by_id, component_node, "performs")
            .into_iter()
            .filter(|node| node.kind == "Effect")
            .map(|node| node.name)
            .collect();
        component.guarantees = outgoing_nodes(core, &node_by_id, component_node, "guarantees")
            .into_iter()
            .filter(|node| node.kind == "Guarantee")
            .map(|node| node.name)
            .collect();
        component.traces = outgoing_nodes(core, &node_by_id, component_node, "records_trace")
            .into_iter()
            .filter(|node| node.kind == "Trace")
            .map(|node| node.name)
            .collect();
        document
            .system_components
            .insert(component.name.clone(), component);
    }

    document
}

fn outgoing_nodes(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    source: &Node,
    edge_kind: &str,
) -> Vec<Node> {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == edge_kind && edge.source == source.id)
        .filter_map(|edge| node_by_id.get(&edge.target).cloned())
        .collect()
}

fn outgoing_edge_payloads(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    source: &Node,
    edge_kind: &str,
    provenance_kind: &str,
) -> Vec<String> {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == edge_kind && edge.source == source.id)
        .filter_map(|edge| {
            let target = node_by_id.get(&edge.target)?;
            let prefix = format!(
                "{}:{}.{}:",
                core_provenance_source_kind(source),
                source.name,
                provenance_kind
            );
            Some(
                edge.attributes
                    .get("provenance")
                    .and_then(|provenance| provenance.strip_prefix(&prefix))
                    .map(str::to_string)
                    .or_else(|| core_node_provenance_payload(core, target, &prefix))
                    .unwrap_or_else(|| target.name.clone()),
            )
        })
        .collect()
}

fn core_provenance_source_kind(source: &Node) -> &'static str {
    match source.kind.as_str() {
        "Tool" => "tool",
        "SystemComponent" => "system_component",
        "Action"
            if source
                .attributes
                .get("kind")
                .is_some_and(|kind| kind == "CompilerPass") =>
        {
            "compiler_pass"
        }
        "Action" => "action",
        _ => "node",
    }
}

fn core_node_provenance_payload(core: &AilCore, node: &Node, prefix: &str) -> Option<String> {
    node_provenance(core, &node.id)
        .and_then(|provenance| provenance.strip_prefix(prefix).map(str::to_string))
}

fn local_core_name(name: &str, owner: &str) -> String {
    name.strip_prefix(&format!("{owner}."))
        .unwrap_or(name)
        .to_string()
}

fn local_target_names(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    source: &Node,
    edge_kind: &str,
    owner: &str,
) -> Vec<String> {
    outgoing_nodes(core, node_by_id, source, edge_kind)
        .into_iter()
        .map(|node| local_core_name(&node.name, owner))
        .collect()
}

fn system_regions_from_core(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    component_name: &str,
) -> Vec<AilSystemResourceRegion> {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "in_region")
        .filter_map(|edge| {
            let resource = node_by_id.get(&edge.source)?;
            let region = node_by_id.get(&edge.target)?;
            if resource.kind != "Resource"
                || region.kind != "Region"
                || !resource.name.starts_with(&format!("{component_name}."))
                || !region.name.starts_with(&format!("{component_name}."))
            {
                return None;
            }
            Some(AilSystemResourceRegion {
                resource_name: local_core_name(&resource.name, component_name),
                region_name: local_core_name(&region.name, component_name),
                provenance: node_provenance(core, &region.id).unwrap_or_default(),
            })
        })
        .collect()
}

fn system_resource_attrs(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    source: &Node,
    edge_kind: &str,
    owner: &str,
) -> Vec<(String, String, String)> {
    outgoing_nodes(core, node_by_id, source, edge_kind)
        .into_iter()
        .map(|node| {
            (
                node.attributes
                    .get("resource")
                    .cloned()
                    .unwrap_or_else(|| local_core_name(&node.name, owner)),
                node.type_name.clone().unwrap_or_default(),
                node_provenance(core, &node.id).unwrap_or_default(),
            )
        })
        .collect()
}

fn system_context_attrs(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    source: &Node,
    edge_kind: &str,
    owner: &str,
) -> Vec<(String, String, String)> {
    outgoing_nodes(core, node_by_id, source, edge_kind)
        .into_iter()
        .map(|node| {
            (
                node.attributes
                    .get("context")
                    .cloned()
                    .unwrap_or_else(|| local_core_name(&node.name, owner)),
                node.type_name.clone().unwrap_or_default(),
                node_provenance(core, &node.id).unwrap_or_default(),
            )
        })
        .collect()
}

fn system_task_attrs(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    source: &Node,
    edge_kind: &str,
    owner: &str,
) -> Vec<(String, String, String)> {
    outgoing_nodes(core, node_by_id, source, edge_kind)
        .into_iter()
        .map(|node| {
            (
                node.attributes
                    .get("task")
                    .cloned()
                    .unwrap_or_else(|| local_core_name(&node.name, owner)),
                node.type_name.clone().unwrap_or_default(),
                node_provenance(core, &node.id).unwrap_or_default(),
            )
        })
        .collect()
}

fn compile_ail_compiler_pass_bytecode(pass: &AilCompilerPass) -> AilBytecodeAction {
    let mut instructions = Vec::new();
    instructions.push(AilBytecodeInstruction::new(
        "PASS_BEGIN",
        &[
            ("pass", pass.name.clone()),
            ("label", pass.label.clone()),
            ("purpose", pass.purpose.clone()),
        ],
    ));
    for input in pass.inputs.values() {
        instructions.push(AilBytecodeInstruction::new(
            "PASS_INPUT",
            &[
                ("name", input.name.clone()),
                ("type", input.type_name.clone()),
            ],
        ));
    }
    for output in pass.outputs.values() {
        instructions.push(AilBytecodeInstruction::new(
            "PASS_OUTPUT",
            &[
                ("name", output.name.clone()),
                ("type", output.type_name.clone()),
            ],
        ));
    }
    for read in &pass.reads {
        instructions.push(AilBytecodeInstruction::new(
            "PASS_READ",
            &[("text", read.clone())],
        ));
    }
    for step in &pass.steps {
        instructions.push(AilBytecodeInstruction::new(
            "PASS_STEP",
            &[("text", step.clone())],
        ));
    }
    for write in &pass.writes {
        instructions.push(AilBytecodeInstruction::new(
            "PASS_WRITE",
            &[("text", write.clone())],
        ));
    }
    if compiler_pass_declares_read_permission_inference(pass) {
        instructions.push(AilBytecodeInstruction::new(
            "CORE_INFER_READ_PERMISSIONS",
            &[
                ("edge", "reads".to_string()),
                ("actor_kinds", "Action,Tool".to_string()),
                ("target_kind", "Field".to_string()),
                ("permission_kind", "Permission".to_string()),
                ("secret_policy", "diagnostic".to_string()),
            ],
        ));
    }
    for guarantee in &pass.guarantees {
        instructions.push(AilBytecodeInstruction::new(
            "ASSERT_GUARANTEE",
            &[("text", guarantee.clone())],
        ));
    }
    for event in &pass.traces {
        instructions.push(AilBytecodeInstruction::new(
            "EMIT_TRACE",
            &[("event", event.clone())],
        ));
    }
    instructions.push(AilBytecodeInstruction::new("RETURN_SUCCESS", &[]));
    AilBytecodeAction {
        name: pass.name.clone(),
        instructions,
    }
}

fn compiler_pass_declares_read_permission_inference(pass: &AilCompilerPass) -> bool {
    pass.writes
        .iter()
        .any(|write| write.to_ascii_lowercase().contains("read permission"))
}

fn compile_ail_system_bytecode(component: &AilSystemComponent) -> AilBytecodeAction {
    let mut instructions = Vec::new();
    instructions.push(AilBytecodeInstruction::new(
        "SYSTEM_BEGIN",
        &[
            ("component", component.name.clone()),
            ("label", component.label.clone()),
        ],
    ));
    for resource in component.resources.values() {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_RESOURCE",
            &[
                ("name", resource.name.clone()),
                ("type", resource.type_name.clone()),
            ],
        ));
    }
    for resource in &component.owned_resources {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_OWNS",
            &[("resource", resource.clone())],
        ));
    }
    for resource in &component.borrowed_resources {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_BORROWS",
            &[("resource", resource.clone())],
        ));
    }
    for resource in &component.mutably_borrowed_resources {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_MUTABLY_BORROWS",
            &[("resource", resource.clone())],
        ));
    }
    for region in &component.resource_regions {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_REGION",
            &[
                ("resource", region.resource_name.clone()),
                ("region", region.region_name.clone()),
            ],
        ));
    }
    for layout in &component.resource_layouts {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_LAYOUT",
            &[
                ("resource", layout.resource_name.clone()),
                ("layout", layout.layout.clone()),
            ],
        ));
    }
    for allocation in &component.resource_allocations {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_ALLOCATION",
            &[
                ("resource", allocation.resource_name.clone()),
                ("placement", allocation.placement.clone()),
            ],
        ));
    }
    for guard in &component.lock_guards {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_LOCK_GUARD",
            &[
                ("resource", guard.resource_name.clone()),
                ("lock", guard.lock_name.clone()),
            ],
        ));
    }
    for context in &component.execution_contexts {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_CONTEXT",
            &[("name", context.name.clone())],
        ));
    }
    for priority in &component.interrupt_priorities {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_INTERRUPT_PRIORITY",
            &[
                ("context", priority.context_name.clone()),
                ("priority", priority.priority.clone()),
            ],
        ));
    }
    for mask in &component.interrupt_masks {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_INTERRUPT_MASK",
            &[
                ("context", mask.context_name.clone()),
                ("mask", mask.mask.clone()),
            ],
        ));
    }
    for task in &component.scheduler_tasks {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_SCHEDULER_TASK",
            &[
                ("task", task.task_name.clone()),
                ("context", task.context_name.clone()),
            ],
        ));
    }
    for priority in &component.scheduler_task_priorities {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_TASK_PRIORITY",
            &[
                ("task", priority.task_name.clone()),
                ("priority", priority.priority.clone()),
            ],
        ));
    }
    for timing in &component.scheduler_task_timings {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_TASK_TIMING",
            &[
                ("task", timing.task_name.clone()),
                ("deadline", timing.deadline.clone()),
                ("budget", timing.budget.clone()),
            ],
        ));
    }
    for capability in &component.capabilities {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_CAPABILITY",
            &[("text", capability.clone())],
        ));
    }
    for effect in &component.effects {
        instructions.push(AilBytecodeInstruction::new(
            "SYSTEM_EFFECT",
            &[("text", effect.clone())],
        ));
    }
    for guarantee in &component.guarantees {
        instructions.push(AilBytecodeInstruction::new(
            "ASSERT_GUARANTEE",
            &[("text", guarantee.clone())],
        ));
    }
    for event in &component.traces {
        instructions.push(AilBytecodeInstruction::new(
            "EMIT_TRACE",
            &[("event", event.clone())],
        ));
    }
    instructions.push(AilBytecodeInstruction::new("RETURN_SUCCESS", &[]));
    AilBytecodeAction {
        name: component.name.clone(),
        instructions,
    }
}

fn compile_ail_tool_bytecode(tool: &AilTool) -> AilBytecodeAction {
    let mut instructions = Vec::new();
    instructions.push(AilBytecodeInstruction::new(
        "TOOL_BEGIN",
        &[("tool", tool.name.clone()), ("label", tool.label.clone())],
    ));
    for requirement in &tool.requirements {
        instructions.push(AilBytecodeInstruction::new(
            "TOOL_REQUIREMENT",
            &[("text", requirement.clone())],
        ));
    }
    for input in tool.inputs.values() {
        instructions.push(AilBytecodeInstruction::new(
            "TOOL_INPUT",
            &[
                ("name", input.name.clone()),
                ("type", input.type_name.clone()),
                ("secret", input.is_secret.to_string()),
            ],
        ));
    }
    for output in tool.outputs.values() {
        instructions.push(AilBytecodeInstruction::new(
            "TOOL_OUTPUT",
            &[
                ("name", output.name.clone()),
                ("type", output.type_name.clone()),
                ("secret", output.is_secret.to_string()),
            ],
        ));
    }
    for read in &tool.reads {
        instructions.push(AilBytecodeInstruction::new(
            "TOOL_READ",
            &[("text", read.clone())],
        ));
    }
    for call in &tool.calls {
        instructions.push(AilBytecodeInstruction::new(
            "TOOL_CALL",
            &[("target", call.clone())],
        ));
    }
    for write in &tool.writes {
        instructions.push(AilBytecodeInstruction::new(
            "TOOL_WRITE",
            &[("text", write.clone())],
        ));
    }
    for permission in &tool.permissions {
        instructions.push(AilBytecodeInstruction::new(
            "TOOL_PERMISSION",
            &[("text", permission.clone())],
        ));
    }
    for approval in &tool.approvals {
        instructions.push(AilBytecodeInstruction::new(
            "TOOL_APPROVAL",
            &[("text", approval.clone())],
        ));
    }
    for protection in &tool.secret_protections {
        instructions.push(AilBytecodeInstruction::new(
            "TOOL_SECRET_PROTECTION",
            &[("text", protection.clone())],
        ));
    }
    for guarantee in &tool.guarantees {
        instructions.push(AilBytecodeInstruction::new(
            "ASSERT_GUARANTEE",
            &[("text", guarantee.clone())],
        ));
    }
    for event in &tool.traces {
        instructions.push(AilBytecodeInstruction::new(
            "EMIT_TRACE",
            &[("event", event.clone())],
        ));
    }
    instructions.push(AilBytecodeInstruction::new("RETURN_SUCCESS", &[]));
    AilBytecodeAction {
        name: tool.name.clone(),
        instructions,
    }
}

fn compile_ail_action_bytecode(document: &AilDocument, action: &AilAction) -> AilBytecodeAction {
    let mut instructions = Vec::new();
    instructions.push(AilBytecodeInstruction::new(
        "ACTION_BEGIN",
        &[("action", action.name.clone())],
    ));
    for requirement in &action.requirements {
        let mut emitted = false;
        if let Some(subject) = existence_requirement_reference(requirement) {
            instructions.push(AilBytecodeInstruction::new(
                "REQUIRE_EXISTS",
                &[
                    ("key", existence_requirement_runtime_key(document, &subject)),
                    ("rule", requirement.clone()),
                    ("failure", "NotFound".to_string()),
                ],
            ));
            emitted = true;
        }
        if let Some((key, allowed_values)) = has_role_requirement(document, requirement) {
            instructions.push(AilBytecodeInstruction::new(
                "REQUIRE_FIELD_IN",
                &[
                    ("key", key),
                    ("values", encode_ail_bytecode_list(&allowed_values)),
                    ("rule", requirement.clone()),
                    ("failure", "RequirementFailed".to_string()),
                ],
            ));
            emitted = true;
        }
        if let Some((key, allowed_values)) = has_permission_requirement(requirement) {
            instructions.push(AilBytecodeInstruction::new(
                "REQUIRE_FIELD_IN",
                &[
                    ("key", key),
                    ("values", encode_ail_bytecode_list(&allowed_values)),
                    ("rule", requirement.clone()),
                    ("failure", "RequirementFailed".to_string()),
                ],
            ));
            emitted = true;
        }
        if let Some(keys) = input_requirement_keys(document, requirement) {
            for key in keys {
                instructions.push(AilBytecodeInstruction::new(
                    "REQUIRE_EXISTS",
                    &[
                        ("key", key),
                        ("rule", requirement.clone()),
                        ("failure", "RequirementFailed".to_string()),
                    ],
                ));
            }
            emitted = true;
        }
        if let Some((source, key)) = field_after_requirement(document, requirement) {
            instructions.push(AilBytecodeInstruction::new(
                "REQUIRE_FIELD_AFTER",
                &[
                    ("source", source),
                    ("key", key),
                    ("rule", requirement.clone()),
                    ("failure", "RequirementFailed".to_string()),
                ],
            ));
            emitted = true;
        }
        if let Some((key, forbidden)) = negative_field_requirement(document, requirement) {
            instructions.push(AilBytecodeInstruction::new(
                "REQUIRE_FIELD_NOT_EQUALS",
                &[
                    ("key", key),
                    ("value", forbidden),
                    ("rule", requirement.clone()),
                    ("failure", "RequirementFailed".to_string()),
                ],
            ));
            emitted = true;
        }
        if let Some((key, allowed_values)) = positive_field_requirement(document, requirement) {
            let failure = failed_requirement_name(document, requirement, &key);
            instructions.push(AilBytecodeInstruction::new(
                "REQUIRE_FIELD_IN",
                &[
                    ("key", key),
                    ("values", encode_ail_bytecode_list(&allowed_values)),
                    ("rule", requirement.clone()),
                    ("failure", failure),
                ],
            ));
            emitted = true;
        }
        if !emitted {
            instructions.push(AilBytecodeInstruction::new(
                "OBSERVE_RULE",
                &[("rule", requirement.clone())],
            ));
        }
    }
    for read in &action.reads {
        if let Some(key) = referenced_runtime_field_key(document, read) {
            instructions.push(AilBytecodeInstruction::new(
                "READ_FIELD",
                &[("key", key), ("text", read.clone())],
            ));
        } else {
            instructions.push(AilBytecodeInstruction::new(
                "READ_EFFECT",
                &[("text", read.clone())],
            ));
        }
    }
    for write in &action.writes {
        if let Some((key, value)) = field_write_assignment(document, write) {
            instructions.push(AilBytecodeInstruction::new(
                "SET_FIELD",
                &[("key", key), ("value", value), ("text", write.clone())],
            ));
        } else if let Some((source, key)) = field_copy_assignment(document, write) {
            instructions.push(AilBytecodeInstruction::new(
                "COPY_FIELD",
                &[("source", source), ("key", key), ("text", write.clone())],
            ));
        } else if let Some(key) = referenced_runtime_field_key(document, write) {
            instructions.push(AilBytecodeInstruction::new(
                "WRITE_FIELD",
                &[("key", key), ("text", write.clone())],
            ));
        } else {
            instructions.push(AilBytecodeInstruction::new(
                "EFFECT",
                &[("text", write.clone())],
            ));
        }
    }
    for guarantee in &action.guarantees {
        instructions.push(AilBytecodeInstruction::new(
            "ASSERT_GUARANTEE",
            &[("text", guarantee.clone())],
        ));
    }
    for event in &action.traces {
        instructions.push(AilBytecodeInstruction::new(
            "EMIT_TRACE",
            &[("event", event.clone())],
        ));
    }
    instructions.push(AilBytecodeInstruction::new("RETURN_SUCCESS", &[]));
    AilBytecodeAction {
        name: action.name.clone(),
        instructions,
    }
}

pub fn render_ail_bytecode(program: &AilBytecodeProgram) -> String {
    format!(
        "{{\"kind\":\"AIL-Bytecode\",\"package\":{},\"version\":{},\"profile\":{},\"actions\":[{}],\"failures\":[{}]}}",
        json_string(&program.package_name),
        json_string(&program.package_version),
        json_string(&program.profile),
        program
            .actions
            .values()
            .map(render_ail_bytecode_action)
            .collect::<Vec<_>>()
            .join(","),
        program
            .failures
            .values()
            .map(render_ail_bytecode_failure)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_ail_bytecode_action(action: &AilBytecodeAction) -> String {
    format!(
        "{{\"action\":{},\"instructions\":[{}]}}",
        json_string(&action.name),
        action
            .instructions
            .iter()
            .map(render_ail_bytecode_instruction)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_ail_bytecode_instruction(instruction: &AilBytecodeInstruction) -> String {
    format!(
        "{{\"opcode\":{},\"operands\":{}}}",
        json_string(&instruction.opcode),
        ail_bytecode_operand_json(&instruction.operands)
    )
}

fn render_ail_bytecode_failure(failure: &AilBytecodeFailure) -> String {
    format!(
        "{{\"failure\":{},\"traces\":{}}}",
        json_string(&failure.name),
        render_json_array(failure.traces.clone())
    )
}

fn ail_bytecode_operand_json(operands: &BTreeMap<String, String>) -> String {
    format!(
        "{{{}}}",
        operands
            .iter()
            .map(|(key, value)| format!("{}:{}", json_string(key), json_string(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub fn parse_ail_bytecode(text: &str) -> Result<AilBytecodeProgram, String> {
    let mut parser = AilJsonParser::new(text);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if !parser.is_finished() {
        return Err("unexpected trailing content in AIL-Bytecode artifact".to_string());
    }
    let root = value
        .as_object()
        .ok_or_else(|| "AIL-Bytecode artifact must be a JSON object".to_string())?;
    let kind = required_json_string(root, "kind")?;
    if kind != "AIL-Bytecode" {
        return Err(format!("expected AIL-Bytecode artifact, got '{kind}'"));
    }
    let mut actions = BTreeMap::new();
    for action_value in required_json_array(root, "actions")? {
        let action_object = action_value
            .as_object()
            .ok_or_else(|| "AIL-Bytecode action must be an object".to_string())?;
        let name = required_json_string(action_object, "action")?.to_string();
        let mut instructions = Vec::new();
        for instruction_value in required_json_array(action_object, "instructions")? {
            let instruction_object = instruction_value
                .as_object()
                .ok_or_else(|| "AIL-Bytecode instruction must be an object".to_string())?;
            let opcode = required_json_string(instruction_object, "opcode")?.to_string();
            let operands = required_json_object(instruction_object, "operands")?
                .iter()
                .map(|(key, value)| {
                    let value = value
                        .as_string()
                        .ok_or_else(|| format!("AIL-Bytecode operand '{key}' must be a string"))?;
                    Ok((key.clone(), value.to_string()))
                })
                .collect::<Result<BTreeMap<_, _>, String>>()?;
            instructions.push(AilBytecodeInstruction { opcode, operands });
        }
        actions.insert(name.clone(), AilBytecodeAction { name, instructions });
    }
    let mut failures = BTreeMap::new();
    for failure_value in required_json_array(root, "failures")? {
        let failure_object = failure_value
            .as_object()
            .ok_or_else(|| "AIL-Bytecode failure must be an object".to_string())?;
        let name = required_json_string(failure_object, "failure")?.to_string();
        let traces = required_json_array(failure_object, "traces")?
            .iter()
            .map(|value| {
                value
                    .as_string()
                    .map(str::to_string)
                    .ok_or_else(|| "AIL-Bytecode failure trace must be a string".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        failures.insert(name.clone(), AilBytecodeFailure { name, traces });
    }
    Ok(AilBytecodeProgram {
        package_name: required_json_string(root, "package")?.to_string(),
        package_version: required_json_string(root, "version")?.to_string(),
        profile: required_json_string(root, "profile")?.to_string(),
        actions,
        failures,
    })
}

pub fn verify_ail_bytecode(program: &AilBytecodeProgram) -> Vec<String> {
    let mut diagnostics = Vec::new();
    for action in program.actions.values() {
        for (index, instruction) in action.instructions.iter().enumerate() {
            let Some(required_operands) =
                ail_bytecode_required_operands(instruction.opcode.as_str())
            else {
                diagnostics.push(format!(
                    "AILBC001 action {} instruction {} uses unknown opcode {}",
                    action.name, index, instruction.opcode
                ));
                continue;
            };
            for operand in required_operands {
                if !instruction.operands.contains_key(*operand) {
                    diagnostics.push(format!(
                        "AILBC002 action {} instruction {} opcode {} is missing operand {}",
                        action.name, index, instruction.opcode, operand
                    ));
                }
            }
        }
    }
    diagnostics
}

fn ail_bytecode_required_operands(opcode: &str) -> Option<&'static [&'static str]> {
    match opcode {
        "ACTION_BEGIN" => Some(&["action"]),
        "REQUIRE_EXISTS" => Some(&["key", "rule", "failure"]),
        "REQUIRE_FIELD_NOT_EQUALS" => Some(&["key", "value", "rule", "failure"]),
        "REQUIRE_FIELD_IN" => Some(&["key", "values", "rule", "failure"]),
        "REQUIRE_FIELD_AFTER" => Some(&["source", "key", "rule", "failure"]),
        "OBSERVE_RULE" => Some(&["rule"]),
        "READ_FIELD" => Some(&["key", "text"]),
        "READ_EFFECT" => Some(&["text"]),
        "SET_FIELD" => Some(&["key", "value", "text"]),
        "COPY_FIELD" => Some(&["source", "key", "text"]),
        "WRITE_FIELD" => Some(&["key", "text"]),
        "EFFECT" => Some(&["text"]),
        "TOOL_BEGIN" => Some(&["tool", "label"]),
        "TOOL_REQUIREMENT" => Some(&["text"]),
        "TOOL_INPUT" => Some(&["name", "type", "secret"]),
        "TOOL_OUTPUT" => Some(&["name", "type", "secret"]),
        "TOOL_READ" => Some(&["text"]),
        "TOOL_CALL" => Some(&["target"]),
        "TOOL_WRITE" => Some(&["text"]),
        "TOOL_PERMISSION" => Some(&["text"]),
        "TOOL_APPROVAL" => Some(&["text"]),
        "TOOL_SECRET_PROTECTION" => Some(&["text"]),
        "SYSTEM_BEGIN" => Some(&["component", "label"]),
        "SYSTEM_RESOURCE" => Some(&["name", "type"]),
        "SYSTEM_OWNS" => Some(&["resource"]),
        "SYSTEM_BORROWS" => Some(&["resource"]),
        "SYSTEM_MUTABLY_BORROWS" => Some(&["resource"]),
        "SYSTEM_REGION" => Some(&["resource", "region"]),
        "SYSTEM_LAYOUT" => Some(&["resource", "layout"]),
        "SYSTEM_ALLOCATION" => Some(&["resource", "placement"]),
        "SYSTEM_LOCK_GUARD" => Some(&["resource", "lock"]),
        "SYSTEM_CONTEXT" => Some(&["name"]),
        "SYSTEM_INTERRUPT_PRIORITY" => Some(&["context", "priority"]),
        "SYSTEM_INTERRUPT_MASK" => Some(&["context", "mask"]),
        "SYSTEM_SCHEDULER_TASK" => Some(&["task", "context"]),
        "SYSTEM_TASK_PRIORITY" => Some(&["task", "priority"]),
        "SYSTEM_TASK_TIMING" => Some(&["task", "deadline", "budget"]),
        "SYSTEM_CAPABILITY" => Some(&["text"]),
        "SYSTEM_EFFECT" => Some(&["text"]),
        "PASS_BEGIN" => Some(&["pass", "label", "purpose"]),
        "PASS_INPUT" => Some(&["name", "type"]),
        "PASS_OUTPUT" => Some(&["name", "type"]),
        "PASS_READ" => Some(&["text"]),
        "PASS_STEP" => Some(&["text"]),
        "PASS_WRITE" => Some(&["text"]),
        "CORE_INFER_READ_PERMISSIONS" => Some(&[
            "edge",
            "actor_kinds",
            "target_kind",
            "permission_kind",
            "secret_policy",
        ]),
        "ASSERT_GUARANTEE" => Some(&["text"]),
        "EMIT_TRACE" => Some(&["event"]),
        "RETURN_SUCCESS" => Some(&[]),
        _ => None,
    }
}

impl AilJsonValue {
    fn as_object(&self) -> Option<&BTreeMap<String, AilJsonValue>> {
        match self {
            AilJsonValue::Object(value) => Some(value),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&[AilJsonValue]> {
        match self {
            AilJsonValue::Array(value) => Some(value),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            AilJsonValue::String(value) => Some(value),
            _ => None,
        }
    }
}

fn required_json_string<'a>(
    object: &'a BTreeMap<String, AilJsonValue>,
    key: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(AilJsonValue::as_string)
        .ok_or_else(|| format!("AIL-Bytecode field '{key}' must be a string"))
}

fn required_json_array<'a>(
    object: &'a BTreeMap<String, AilJsonValue>,
    key: &str,
) -> Result<&'a [AilJsonValue], String> {
    object
        .get(key)
        .and_then(AilJsonValue::as_array)
        .ok_or_else(|| format!("AIL-Bytecode field '{key}' must be an array"))
}

fn required_json_object<'a>(
    object: &'a BTreeMap<String, AilJsonValue>,
    key: &str,
) -> Result<&'a BTreeMap<String, AilJsonValue>, String> {
    object
        .get(key)
        .and_then(AilJsonValue::as_object)
        .ok_or_else(|| format!("AIL-Bytecode field '{key}' must be an object"))
}

struct AilJsonParser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> AilJsonParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            chars: text.chars().peekable(),
        }
    }

    fn parse_value(&mut self) -> Result<AilJsonValue, String> {
        self.skip_whitespace();
        match self.chars.peek().copied() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string().map(AilJsonValue::String),
            Some(ch) => Err(format!("unexpected JSON value starting with '{ch}'")),
            None => Err("unexpected end of JSON input".to_string()),
        }
    }

    fn parse_object(&mut self) -> Result<AilJsonValue, String> {
        self.expect_char('{')?;
        let mut object = BTreeMap::new();
        self.skip_whitespace();
        if self.consume_char('}') {
            return Ok(AilJsonValue::Object(object));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_char(':')?;
            let value = self.parse_value()?;
            object.insert(key, value);
            self.skip_whitespace();
            if self.consume_char('}') {
                break;
            }
            self.expect_char(',')?;
        }
        Ok(AilJsonValue::Object(object))
    }

    fn parse_array(&mut self) -> Result<AilJsonValue, String> {
        self.expect_char('[')?;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.consume_char(']') {
            return Ok(AilJsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.consume_char(']') {
                break;
            }
            self.expect_char(',')?;
        }
        Ok(AilJsonValue::Array(values))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect_char('"')?;
        let mut value = String::new();
        while let Some(ch) = self.chars.next() {
            match ch {
                '"' => return Ok(value),
                '\\' => value.push(self.parse_escape()?),
                ch => value.push(ch),
            }
        }
        Err("unterminated JSON string".to_string())
    }

    fn parse_escape(&mut self) -> Result<char, String> {
        match self.chars.next() {
            Some('"') => Ok('"'),
            Some('\\') => Ok('\\'),
            Some('/') => Ok('/'),
            Some('b') => Ok('\u{0008}'),
            Some('f') => Ok('\u{000c}'),
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('u') => {
                let mut hex = String::new();
                for _ in 0..4 {
                    let Some(ch) = self.chars.next() else {
                        return Err("incomplete JSON unicode escape".to_string());
                    };
                    hex.push(ch);
                }
                let code = u32::from_str_radix(&hex, 16)
                    .map_err(|_| format!("invalid JSON unicode escape '\\u{hex}'"))?;
                char::from_u32(code).ok_or_else(|| format!("invalid JSON codepoint {code}"))
            }
            Some(ch) => Err(format!("unsupported JSON escape '\\{ch}'")),
            None => Err("incomplete JSON escape".to_string()),
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        match self.chars.next() {
            Some(ch) if ch == expected => Ok(()),
            Some(ch) => Err(format!("expected '{expected}', got '{ch}'")),
            None => Err(format!("expected '{expected}', got end of input")),
        }
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.chars.peek().copied() == Some(expected) {
            self.chars.next();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while self.chars.peek().is_some_and(|ch| ch.is_whitespace()) {
            self.chars.next();
        }
    }

    fn is_finished(&mut self) -> bool {
        self.chars.peek().is_none()
    }
}

pub fn run_ail_bytecode_action(
    program: &AilBytecodeProgram,
    action_name: &str,
    runtime_state: BTreeMap<String, String>,
) -> Result<AilRunResult, String> {
    let diagnostics = verify_ail_bytecode(program);
    if !diagnostics.is_empty() {
        return Err(format!("invalid AIL bytecode:\n{}", diagnostics.join("\n")));
    }
    let action = program
        .actions
        .get(action_name)
        .ok_or_else(|| format!("unknown AIL bytecode action '{action_name}'"))?;
    let mut final_state = runtime_state;
    let mut trace = Vec::new();
    for instruction in &action.instructions {
        match instruction.opcode.as_str() {
            "ACTION_BEGIN" => {
                let action = ail_bytecode_operand(instruction, "action");
                trace.push(format!("action {action} started"));
            }
            "REQUIRE_EXISTS" => {
                let key = ail_bytecode_operand(instruction, "key");
                let rule = ail_bytecode_operand(instruction, "rule");
                if !final_state.contains_key(key) {
                    return Ok(failed_bytecode_run(
                        program,
                        final_state,
                        trace,
                        ail_bytecode_operand(instruction, "failure"),
                    ));
                }
                trace.push(format!("rule passed: {rule}"));
            }
            "REQUIRE_FIELD_NOT_EQUALS" => {
                let key = ail_bytecode_operand(instruction, "key");
                let value = ail_bytecode_operand(instruction, "value");
                let rule = ail_bytecode_operand(instruction, "rule");
                if final_state.get(key).is_some_and(|current| current == value) {
                    return Ok(failed_bytecode_run(
                        program,
                        final_state,
                        trace,
                        ail_bytecode_operand(instruction, "failure"),
                    ));
                }
                trace.push(format!("rule passed: {rule}"));
            }
            "REQUIRE_FIELD_IN" => {
                let key = ail_bytecode_operand(instruction, "key");
                let values = decode_ail_bytecode_list(ail_bytecode_operand(instruction, "values"));
                let rule = ail_bytecode_operand(instruction, "rule");
                if !final_state
                    .get(key)
                    .is_some_and(|current| values.iter().any(|value| current == value))
                {
                    return Ok(failed_bytecode_run(
                        program,
                        final_state,
                        trace,
                        ail_bytecode_operand(instruction, "failure"),
                    ));
                }
                trace.push(format!("rule passed: {rule}"));
            }
            "REQUIRE_FIELD_AFTER" => {
                let source = ail_bytecode_operand(instruction, "source");
                let key = ail_bytecode_operand(instruction, "key");
                let rule = ail_bytecode_operand(instruction, "rule");
                if final_state
                    .get(source)
                    .zip(final_state.get(key))
                    .is_none_or(|(left, right)| left <= right)
                {
                    return Ok(failed_bytecode_run(
                        program,
                        final_state,
                        trace,
                        ail_bytecode_operand(instruction, "failure"),
                    ));
                }
                trace.push(format!("rule passed: {rule}"));
            }
            "OBSERVE_RULE" => {
                trace.push(format!(
                    "rule observed: {}",
                    ail_bytecode_operand(instruction, "rule")
                ));
            }
            "READ_FIELD" => {
                trace.push(format!("read {}", ail_bytecode_operand(instruction, "key")));
            }
            "READ_EFFECT" => {
                trace.push(format!(
                    "read {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "SET_FIELD" => {
                let key = ail_bytecode_operand(instruction, "key").to_string();
                let value = ail_bytecode_operand(instruction, "value").to_string();
                final_state.insert(key.clone(), value.clone());
                trace.push(format!("write {key}={value}"));
            }
            "COPY_FIELD" => {
                let source = ail_bytecode_operand(instruction, "source");
                let key = ail_bytecode_operand(instruction, "key").to_string();
                if let Some(value) = final_state.get(source).cloned() {
                    final_state.insert(key.clone(), value);
                }
                trace.push(format!("write {key}"));
            }
            "WRITE_FIELD" => {
                trace.push(format!(
                    "write {}",
                    ail_bytecode_operand(instruction, "key")
                ));
            }
            "EFFECT" => {
                trace.push(format!(
                    "effect {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "TOOL_BEGIN" => {
                trace.push(format!(
                    "tool {} started",
                    ail_bytecode_operand(instruction, "label")
                ));
            }
            "TOOL_REQUIREMENT" => {
                trace.push(format!(
                    "tool requirement {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "TOOL_INPUT" => {
                trace.push(format!(
                    "tool input {}:{}",
                    ail_bytecode_operand(instruction, "name"),
                    ail_bytecode_operand(instruction, "type")
                ));
            }
            "TOOL_OUTPUT" => {
                trace.push(format!(
                    "tool output {}:{}",
                    ail_bytecode_operand(instruction, "name"),
                    ail_bytecode_operand(instruction, "type")
                ));
            }
            "TOOL_READ" => {
                trace.push(format!(
                    "tool read {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "TOOL_CALL" => {
                trace.push(format!(
                    "tool call {}",
                    ail_bytecode_operand(instruction, "target")
                ));
            }
            "TOOL_WRITE" => {
                trace.push(format!(
                    "tool write {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "TOOL_PERMISSION" => {
                trace.push(format!(
                    "tool permission {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "TOOL_APPROVAL" => {
                trace.push(format!(
                    "tool approval {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "TOOL_SECRET_PROTECTION" => {
                trace.push(format!(
                    "tool secret protection {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "SYSTEM_BEGIN" => {
                trace.push(format!(
                    "system component {} started",
                    ail_bytecode_operand(instruction, "label")
                ));
            }
            "SYSTEM_RESOURCE" => {
                trace.push(format!(
                    "system resource {}:{}",
                    ail_bytecode_operand(instruction, "name"),
                    ail_bytecode_operand(instruction, "type")
                ));
            }
            "SYSTEM_OWNS" => {
                trace.push(format!(
                    "system owns {}",
                    ail_bytecode_operand(instruction, "resource")
                ));
            }
            "SYSTEM_BORROWS" => {
                trace.push(format!(
                    "system borrows {}",
                    ail_bytecode_operand(instruction, "resource")
                ));
            }
            "SYSTEM_MUTABLY_BORROWS" => {
                trace.push(format!(
                    "system mutably borrows {}",
                    ail_bytecode_operand(instruction, "resource")
                ));
            }
            "SYSTEM_REGION" => {
                trace.push(format!(
                    "system places {} in {}",
                    ail_bytecode_operand(instruction, "resource"),
                    ail_bytecode_operand(instruction, "region")
                ));
            }
            "SYSTEM_LAYOUT" => {
                trace.push(format!(
                    "system layout {} {}",
                    ail_bytecode_operand(instruction, "resource"),
                    ail_bytecode_operand(instruction, "layout")
                ));
            }
            "SYSTEM_ALLOCATION" => {
                trace.push(format!(
                    "system allocation {} {}",
                    ail_bytecode_operand(instruction, "resource"),
                    ail_bytecode_operand(instruction, "placement")
                ));
            }
            "SYSTEM_LOCK_GUARD" => {
                trace.push(format!(
                    "system lock guard {} with {}",
                    ail_bytecode_operand(instruction, "resource"),
                    ail_bytecode_operand(instruction, "lock")
                ));
            }
            "SYSTEM_CONTEXT" => {
                trace.push(format!(
                    "system context {}",
                    ail_bytecode_operand(instruction, "name")
                ));
            }
            "SYSTEM_INTERRUPT_PRIORITY" => {
                trace.push(format!(
                    "system interrupt priority {} {}",
                    ail_bytecode_operand(instruction, "context"),
                    ail_bytecode_operand(instruction, "priority")
                ));
            }
            "SYSTEM_INTERRUPT_MASK" => {
                trace.push(format!(
                    "system interrupt mask {} {}",
                    ail_bytecode_operand(instruction, "context"),
                    ail_bytecode_operand(instruction, "mask")
                ));
            }
            "SYSTEM_SCHEDULER_TASK" => {
                trace.push(format!(
                    "system scheduler task {} in {}",
                    ail_bytecode_operand(instruction, "task"),
                    ail_bytecode_operand(instruction, "context")
                ));
            }
            "SYSTEM_TASK_PRIORITY" => {
                trace.push(format!(
                    "system task priority {} {}",
                    ail_bytecode_operand(instruction, "task"),
                    ail_bytecode_operand(instruction, "priority")
                ));
            }
            "SYSTEM_TASK_TIMING" => {
                trace.push(format!(
                    "system task timing {} deadline {} budget {}",
                    ail_bytecode_operand(instruction, "task"),
                    ail_bytecode_operand(instruction, "deadline"),
                    ail_bytecode_operand(instruction, "budget")
                ));
            }
            "SYSTEM_CAPABILITY" => {
                trace.push(format!(
                    "system capability {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "SYSTEM_EFFECT" => {
                trace.push(format!(
                    "system effect {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "PASS_BEGIN" => {
                trace.push(format!(
                    "compiler pass {} started",
                    ail_bytecode_operand(instruction, "label")
                ));
            }
            "PASS_INPUT" => {
                trace.push(format!(
                    "input {}:{}",
                    ail_bytecode_operand(instruction, "name"),
                    ail_bytecode_operand(instruction, "type")
                ));
            }
            "PASS_OUTPUT" => {
                trace.push(format!(
                    "output {}:{}",
                    ail_bytecode_operand(instruction, "name"),
                    ail_bytecode_operand(instruction, "type")
                ));
            }
            "PASS_READ" => {
                trace.push(format!(
                    "pass read {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "PASS_STEP" => {
                trace.push(format!(
                    "pass step {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "PASS_WRITE" => {
                trace.push(format!(
                    "pass write {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "CORE_INFER_READ_PERMISSIONS" => {
                trace.push("core transform infer read permissions".to_string());
            }
            "ASSERT_GUARANTEE" => {
                trace.push(format!(
                    "guarantee checked: {}",
                    ail_bytecode_operand(instruction, "text")
                ));
            }
            "EMIT_TRACE" => {
                trace.push(format!(
                    "trace {}",
                    ail_bytecode_operand(instruction, "event")
                ));
            }
            "RETURN_SUCCESS" => {
                return Ok(AilRunResult {
                    status: "succeeded".to_string(),
                    failure: None,
                    final_state,
                    trace,
                });
            }
            opcode => return Err(format!("unknown AIL bytecode opcode '{opcode}'")),
        }
    }
    Ok(AilRunResult {
        status: "succeeded".to_string(),
        failure: None,
        final_state,
        trace,
    })
}

pub fn run_ail_compiler_pass_on_core(
    program: &AilBytecodeProgram,
    action_name: &str,
    core: &AilCore,
) -> Result<AilCompilerPassRunResult, String> {
    if program.profile != "Compiler" {
        return Err(format!(
            "AIL compiler pass runner requires Compiler bytecode, not {}",
            program.profile
        ));
    }
    let action = program
        .actions
        .get(action_name)
        .ok_or_else(|| format!("unknown AIL bytecode action '{action_name}'"))?;
    let mut run = run_ail_bytecode_action(
        program,
        action_name,
        BTreeMap::from([
            ("input graph".to_string(), render_ail_core(core)),
            ("package policy".to_string(), "infer reads".to_string()),
        ]),
    )?;
    let mut output = core.clone();
    for instruction in &action.instructions {
        if instruction.opcode == "CORE_INFER_READ_PERMISSIONS" {
            infer_read_permissions(&mut output, action_name, &mut run.trace);
        }
    }
    run.final_state
        .insert("output graph".to_string(), render_ail_core(&output));
    Ok(AilCompilerPassRunResult { core: output, run })
}

fn infer_read_permissions(core: &mut AilCore, pass_name: &str, trace: &mut Vec<String>) {
    let node_by_id = graph_node_by_id(core);
    let read_edges = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "reads")
        .filter_map(|edge| {
            let source = node_by_id.get(&edge.source)?;
            let target = node_by_id.get(&edge.target)?;
            (matches!(source.kind.as_str(), "Action" | "Tool") && target.kind == "Field")
                .then(|| (source.clone(), target.clone()))
        })
        .collect::<Vec<_>>();

    for (source, target) in read_edges {
        if is_secret_ail_node(&target) {
            trace.push(format!(
                "compiler diagnostic secret read needs confirmation: {}",
                target.name
            ));
            continue;
        }
        let permission_name = format!("read {}", target.name);
        if source_has_permission(&core.graph, &source.id, &permission_name) {
            continue;
        }
        let permission =
            core.graph
                .add_node("Permission", permission_name.clone(), None, BTreeMap::new());
        core.graph.add_edge(
            "requires",
            &source,
            &permission,
            attr(&[(
                "provenance",
                &format!("compiler_pass:{pass_name}.permission:{permission_name}"),
            )]),
        );
        attach_provenance(
            &mut core.graph,
            &permission,
            format!("compiler_pass:{pass_name}.permission:{permission_name}"),
        );
        trace.push(format!(
            "compiler pass {pass_name} added Permission {permission_name} to {}",
            source.name
        ));
    }
}

fn source_has_permission(graph: &Graph, source_id: &str, permission_name: &str) -> bool {
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.clone()))
        .collect::<BTreeMap<_, _>>();
    graph.edges.iter().any(|edge| {
        edge.kind == "requires"
            && edge.source == source_id
            && node_by_id
                .get(&edge.target)
                .is_some_and(|node| node.kind == "Permission" && node.name == permission_name)
    })
}

fn is_secret_ail_node(node: &Node) -> bool {
    node.attributes
        .get("secret")
        .is_some_and(|value| value == "true")
        || node.type_name.as_deref().is_some_and(type_contains_secret)
}

fn failed_bytecode_run(
    program: &AilBytecodeProgram,
    final_state: BTreeMap<String, String>,
    mut trace: Vec<String>,
    failure_name: &str,
) -> AilRunResult {
    trace.push(format!("failure {failure_name}"));
    if let Some(failure) = program.failures.get(failure_name) {
        for event in &failure.traces {
            trace.push(format!("trace {event}"));
        }
    }
    AilRunResult {
        status: "failed".to_string(),
        failure: Some(failure_name.to_string()),
        final_state,
        trace,
    }
}

fn ail_bytecode_operand<'a>(instruction: &'a AilBytecodeInstruction, key: &str) -> &'a str {
    instruction
        .operands
        .get(key)
        .map(String::as_str)
        .unwrap_or("")
}

fn encode_ail_bytecode_list(values: &[String]) -> String {
    values.join("\u{1f}")
}

fn decode_ail_bytecode_list(values: &str) -> Vec<String> {
    values
        .split('\u{1f}')
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn draft_ail_spec(
    package: &AilPackage,
    user_prompt: &str,
    endpoint: &str,
) -> Result<AilDraftResult, String> {
    let prompt = build_ail_draft_prompt(package, user_prompt);
    let spec_text = crate::llm::invoke_llm_text_for_artifact(
        endpoint,
        &prompt,
        "AIL-Spec Canonical",
        package.metadata.profile.as_str(),
    )?;
    Ok(check_ail_draft_spec(package, spec_text))
}

pub fn draft_ail_requirements(
    package: &AilPackage,
    user_prompt: &str,
    endpoint: &str,
) -> Result<String, String> {
    let prompt = build_ail_requirements_prompt(package, user_prompt);
    crate::llm::invoke_llm_text_for_artifact(
        endpoint,
        &prompt,
        "AIL-Requirements",
        package.metadata.profile.as_str(),
    )
}

pub fn repair_ail_requirements_from_diagnostics(
    package: &AilPackage,
    user_prompt: &str,
    previous_requirements_text: &str,
    diagnostics: &[AilDiagnostic],
    endpoint: &str,
) -> Result<String, String> {
    let prompt = build_ail_requirements_repair_prompt(
        package,
        user_prompt,
        previous_requirements_text,
        diagnostics,
    );
    crate::llm::invoke_llm_text_for_artifact(
        endpoint,
        &prompt,
        "AIL-Requirements",
        package.metadata.profile.as_str(),
    )
}

pub fn check_ail_requirements(package: &AilPackage, requirements_text: &str) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    let trimmed = requirements_text.trim();
    let lowered = trimmed.to_ascii_lowercase();
    let bullets = trimmed
        .lines()
        .filter(|line| line.trim_start().starts_with("- "))
        .collect::<Vec<_>>();

    if !lowered.starts_with("ail-requirements:") {
        diagnostics.push(
            AilDiagnostic::error(
                "AILR001",
                "requirements artifact must start with AIL-Requirements:",
            )
            .with_repair_suggestion("Return only an AIL-Requirements artifact with that header."),
        );
    }

    if bullets.len() < 3 {
        diagnostics.push(
            AilDiagnostic::error(
                "AILR002",
                "requirements artifact needs at least three requirement bullets",
            )
            .with_repair_suggestion(
                "Add concise bullets that cover the package surface, behavior, and audit expectations.",
            ),
        );
    }

    for topic in required_requirements_topics(package.metadata.profile.as_str()) {
        if !requirements_mentions_any(&lowered, topic.terms) {
            diagnostics.push(
                AilDiagnostic::error(topic.code, topic.message)
                    .with_repair_suggestion(topic.repair),
            );
        }
    }

    diagnostics
}

pub fn draft_ail_spec_from_requirements(
    package: &AilPackage,
    user_prompt: &str,
    requirements_text: &str,
    endpoint: &str,
) -> Result<AilDraftResult, String> {
    let grounded_prompt = format!("{user_prompt}\n\nDRAFT REQUIREMENTS:\n{requirements_text}");
    let prompt = build_ail_draft_prompt(package, &grounded_prompt);
    let spec_text = crate::llm::invoke_llm_text_for_artifact(
        endpoint,
        &prompt,
        "AIL-Spec Canonical",
        package.metadata.profile.as_str(),
    )?;
    Ok(check_ail_draft_spec_with_requirements(
        package,
        spec_text,
        Some(requirements_text),
    ))
}

pub fn repair_ail_spec_from_diagnostics(
    package: &AilPackage,
    user_prompt: &str,
    requirements_text: &str,
    previous_spec_text: &str,
    diagnostics: &[AilDiagnostic],
    endpoint: &str,
) -> Result<AilDraftResult, String> {
    let prompt = build_ail_repair_prompt(
        package,
        user_prompt,
        requirements_text,
        previous_spec_text,
        diagnostics,
    );
    let spec_text = crate::llm::invoke_llm_text_for_artifact(
        endpoint,
        &prompt,
        "AIL-Spec Canonical",
        package.metadata.profile.as_str(),
    )?;
    Ok(check_ail_draft_spec_with_requirements(
        package,
        spec_text,
        Some(requirements_text),
    ))
}

fn check_ail_draft_spec(package: &AilPackage, spec_text: String) -> AilDraftResult {
    check_ail_draft_spec_with_requirements(package, spec_text, None)
}

fn check_ail_draft_spec_with_requirements(
    package: &AilPackage,
    spec_text: String,
    requirements_text: Option<&str>,
) -> AilDraftResult {
    let diagnostics = match parse_ail_package_spec_text(package, &spec_text) {
        Ok(document) => {
            let mut diagnostics =
                check_ail_core_diagnostics(&elaborate_ail_core(package, &document));
            if let Some(requirements_text) = requirements_text {
                diagnostics.extend(check_ail_spec_preserves_requirement_permissions(
                    &document,
                    requirements_text,
                ));
                diagnostics.extend(check_ail_spec_preserves_requirement_failures(
                    &document,
                    requirements_text,
                ));
            }
            diagnostics
        }
        Err(error) => vec![AilDiagnostic::error(
            "AIL000",
            format!("parse error: {error}"),
        )],
    };
    AilDraftResult {
        spec_text,
        diagnostics,
    }
}

fn check_ail_spec_preserves_requirement_permissions(
    document: &AilDocument,
    requirements_text: &str,
) -> Vec<AilDiagnostic> {
    let mut missing_actions = BTreeSet::new();
    for line in requirements_text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- "))
    {
        let lowered = line.to_ascii_lowercase();
        if !mentions_permission_requirement(&lowered) {
            continue;
        }
        let compact_line = compact_requirement_match_text(line);
        for action in document.actions.values() {
            let compact_name = compact_requirement_match_text(&action.name);
            let compact_label = compact_requirement_match_text(&action.label);
            if (compact_line.contains(&compact_name) || compact_line.contains(&compact_label))
                && !action_preserves_permission_requirement(action)
            {
                missing_actions.insert(action.name.clone());
            }
        }
    }
    missing_actions
        .into_iter()
        .map(|action_name| {
            AilDiagnostic::error(
                "AILR011",
                format!("spec is missing permission requirement for action {action_name}"),
            )
            .with_repair_suggestion(format!(
                "Add an explicit permission, role, approval, access, or forbidden-state requirement to action {action_name}."
            ))
        })
        .collect()
}

fn check_ail_spec_preserves_requirement_failures(
    document: &AilDocument,
    requirements_text: &str,
) -> Vec<AilDiagnostic> {
    let mut missing_failures = BTreeSet::new();
    for failure_name in requirements_text
        .lines()
        .map(str::trim)
        .filter_map(requirement_failure_name)
    {
        if !document.failures.contains_key(&failure_name) {
            missing_failures.insert(failure_name);
        }
    }
    missing_failures
        .into_iter()
        .map(|failure_name| {
            AilDiagnostic::error(
                "AILR012",
                format!("spec is missing required Failure {failure_name}"),
            )
            .with_repair_suggestion(format!(
                "Add a 'Failure {failure_name} happens when ...' section with handling and trace bullets."
            ))
        })
        .collect()
}

fn requirement_failure_name(line: &str) -> Option<String> {
    let bullet = line.strip_prefix("- ")?.trim();
    let (failure_name, _) = parse_failure_header(bullet)?;
    Some(failure_name)
}

fn action_preserves_permission_requirement(action: &AilAction) -> bool {
    action
        .requirements
        .iter()
        .any(|requirement| mentions_permission_requirement(&requirement.to_ascii_lowercase()))
}

fn mentions_permission_requirement(text: &str) -> bool {
    [
        "permission",
        "approval",
        "authorized",
        "authorised",
        "access",
        "forbidden",
        " role",
    ]
    .iter()
    .any(|term| text.contains(term))
}

fn compact_requirement_match_text(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn run_ail_conformance(package: &AilPackage) -> Result<AilConformanceResult, String> {
    let document = parse_ail_package_document(package)?;
    let core = elaborate_ail_core(package, &document);
    let accepted_diagnostics = check_ail_core_diagnostics(&core);
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let accepted_dir = package.root.join("examples").join("accepted");
    let rejected_dir = package.root.join("examples").join("rejected");

    if accepted_dir.exists() {
        let mut paths = fs::read_dir(&accepted_dir)
            .map_err(|error| format!("failed to read {}: {error}", accepted_dir.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("failed to read {}: {error}", accepted_dir.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.retain(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".ail-spec.md"))
        });
        paths.sort();

        for path in paths {
            let fixture = file_name_or_display(&path);
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            let diagnostics = match parse_ail_package_spec_text(package, &text) {
                Ok(document) => check_ail_core_diagnostics(&elaborate_ail_core(package, &document)),
                Err(error) => vec![AilDiagnostic::error(
                    "AIL000",
                    format!("parse error: {error}"),
                )],
            };
            accepted.push(AilAcceptedConformanceResult {
                fixture,
                diagnostics,
            });
        }
    }

    if rejected_dir.exists() {
        let mut paths = fs::read_dir(&rejected_dir)
            .map_err(|error| format!("failed to read {}: {error}", rejected_dir.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("failed to read {}: {error}", rejected_dir.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.retain(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".ail-spec.md"))
        });
        paths.sort();

        for path in paths {
            let fixture = file_name_or_display(&path);
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            let diagnostics = match parse_ail_package_spec_text(package, &text) {
                Ok(document) => check_ail_core_diagnostics(&elaborate_ail_core(package, &document)),
                Err(error) => vec![AilDiagnostic::error(
                    "AIL000",
                    format!("parse error: {error}"),
                )],
            };
            rejected.push(AilRejectedConformanceResult {
                fixture,
                diagnostics,
            });
        }
    }

    Ok(AilConformanceResult {
        package_name: package.metadata.name.clone(),
        accepted_fixture: file_name_or_display(&package.spec_path),
        accepted_diagnostics,
        accepted,
        rejected,
    })
}

fn parse_package_metadata(text: &str) -> Result<AilPackageMetadata, String> {
    let mut values = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        values.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    let name = required_metadata(&values, "name")?;
    let version = values
        .get("version")
        .cloned()
        .unwrap_or_else(|| "0.1.0".to_string());
    let profile = required_metadata(&values, "profile")?;
    let entry = required_metadata(&values, "entry")?;
    let features = values
        .get("features")
        .map(|features| {
            features
                .split(',')
                .map(str::trim)
                .filter(|feature| !feature.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_else(Vec::new);
    let imports = values
        .get("imports")
        .map(|imports| parse_import_specs(imports))
        .transpose()?
        .unwrap_or_default();
    let conformance = values
        .get("conformance")
        .cloned()
        .unwrap_or_else(|| "draft".to_string());
    let base_llm_endpoint = values
        .get("base_llm_endpoint")
        .cloned()
        .unwrap_or_else(|| DEFAULT_BASE_LLM_ENDPOINT.to_string());
    Ok(AilPackageMetadata {
        name,
        version,
        profile,
        entry,
        features,
        imports,
        conformance,
        base_llm_endpoint,
    })
}

fn parse_import_specs(text: &str) -> Result<Vec<AilImportSpec>, String> {
    text.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let Some((path, alias)) = entry.split_once(" as ") else {
                return Err(format!("AIL import '{entry}' must use '<path> as <Alias>'"));
            };
            let path = path.trim();
            let alias = alias.trim();
            if path.is_empty() || alias.is_empty() {
                return Err(format!("AIL import '{entry}' must use '<path> as <Alias>'"));
            }
            Ok(AilImportSpec {
                path: path.to_string(),
                alias: alias.to_string(),
            })
        })
        .collect()
}

fn merge_ail_import(target: &mut AilDocument, imported: AilDocument) {
    for (name, thing) in imported.things {
        target.things.insert(name, thing);
    }
    for view in imported.application.views {
        if !target.application.views.contains(&view) {
            target.application.views.push(view);
        }
    }
    for (name, tool) in imported.tools {
        target.tools.insert(name, tool);
    }
    for (name, pass) in imported.compiler_passes {
        target.compiler_passes.insert(name, pass);
    }
    for (name, component) in imported.system_components {
        target.system_components.insert(name, component);
    }
    for (name, action) in imported.actions {
        target.actions.insert(name, action);
    }
    for (name, failure) in imported.failures {
        target.failures.insert(name, failure);
    }
}

fn set_graph_node_attribute(graph: &mut Graph, node_id: &str, key: &str, value: &str) {
    if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == node_id) {
        node.attributes.insert(key.to_string(), value.to_string());
    }
}

fn namespace_ail_document(document: &AilDocument, alias: &str) -> AilDocument {
    let thing_names = document.things.keys().cloned().collect::<Vec<_>>();
    let failure_names = document.failures.keys().cloned().collect::<Vec<_>>();
    let mut namespaced = AilDocument {
        application: AilApplication {
            name: format!("{alias}.{}", document.application.name),
            purpose: document.application.purpose.clone(),
            users: document.application.users.clone(),
            views: document
                .application
                .views
                .iter()
                .map(|view| format!("{alias}: {view}"))
                .collect(),
        },
        things: BTreeMap::new(),
        tools: BTreeMap::new(),
        compiler_passes: BTreeMap::new(),
        system_components: BTreeMap::new(),
        actions: BTreeMap::new(),
        failures: BTreeMap::new(),
    };

    for thing in document.things.values() {
        let thing_name = qualify_name(alias, &thing.name);
        let mut fields = BTreeMap::new();
        for field in thing.fields.values() {
            fields.insert(
                field.name.clone(),
                AilField {
                    name: field.name.clone(),
                    type_name: qualify_type_name(&field.type_name, alias, &thing_names),
                    is_secret: field.is_secret,
                    provenance: format!("field:{thing_name}.{}", field.name),
                },
            );
        }
        namespaced.things.insert(
            thing_name.clone(),
            AilThing {
                name: thing_name.clone(),
                fields,
                provenance: format!("thing:{thing_name}"),
            },
        );
    }

    for action in document.actions.values() {
        let action_name = qualify_name(alias, &action.name);
        namespaced.actions.insert(
            action_name.clone(),
            AilAction {
                name: action_name.clone(),
                label: format!("{alias}.{}", action.label),
                trigger: qualify_reference_text(&action.trigger, alias, &thing_names),
                requirements: action
                    .requirements
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                reads: action
                    .reads
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                writes: action
                    .writes
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                failures: action
                    .failures
                    .iter()
                    .map(|failure| qualify_failure_reference(failure, alias, &failure_names))
                    .collect(),
                guarantees: action.guarantees.clone(),
                traces: action
                    .traces
                    .iter()
                    .map(|trace| qualify_name(alias, trace))
                    .collect(),
                secret_protections: action
                    .secret_protections
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                provenance: format!("action:{action_name}"),
            },
        );
    }

    for tool in document.tools.values() {
        let tool_name = qualify_name(alias, &tool.name);
        namespaced.tools.insert(
            tool_name.clone(),
            AilTool {
                name: tool_name.clone(),
                label: format!("{alias}.{}", tool.label),
                requirements: tool
                    .requirements
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                inputs: namespace_tool_slots(alias, &tool_name, &tool.inputs, &thing_names),
                outputs: namespace_tool_slots(alias, &tool_name, &tool.outputs, &thing_names),
                reads: tool
                    .reads
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                writes: tool
                    .writes
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                calls: tool.calls.clone(),
                permissions: tool
                    .permissions
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                approvals: tool
                    .approvals
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                failures: tool
                    .failures
                    .iter()
                    .map(|failure| qualify_failure_reference(failure, alias, &failure_names))
                    .collect(),
                guarantees: tool.guarantees.clone(),
                traces: tool
                    .traces
                    .iter()
                    .map(|trace| qualify_name(alias, trace))
                    .collect(),
                secret_protections: tool
                    .secret_protections
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                provenance: format!("tool:{tool_name}"),
            },
        );
    }

    for pass in document.compiler_passes.values() {
        let pass_name = qualify_name(alias, &pass.name);
        namespaced.compiler_passes.insert(
            pass_name.clone(),
            AilCompilerPass {
                name: pass_name.clone(),
                label: format!("{alias}.{}", pass.label),
                purpose: pass.purpose.clone(),
                inputs: namespace_pass_values(alias, &pass_name, &pass.inputs, &thing_names),
                outputs: namespace_pass_values(alias, &pass_name, &pass.outputs, &thing_names),
                reads: pass
                    .reads
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                writes: pass
                    .writes
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                steps: pass.steps.clone(),
                failures: pass
                    .failures
                    .iter()
                    .map(|failure| qualify_failure_reference(failure, alias, &failure_names))
                    .collect(),
                guarantees: pass.guarantees.clone(),
                traces: pass
                    .traces
                    .iter()
                    .map(|trace| qualify_name(alias, trace))
                    .collect(),
                provenance: format!("compiler_pass:{pass_name}"),
            },
        );
    }

    for component in document.system_components.values() {
        let component_name = qualify_name(alias, &component.name);
        namespaced.system_components.insert(
            component_name.clone(),
            AilSystemComponent {
                name: component_name.clone(),
                label: format!("{alias}.{}", component.label),
                resources: namespace_system_resources(
                    alias,
                    &component_name,
                    &component.resources,
                    &thing_names,
                ),
                owned_resources: component
                    .owned_resources
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                borrowed_resources: component
                    .borrowed_resources
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                mutably_borrowed_resources: component
                    .mutably_borrowed_resources
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                resource_regions: namespace_system_resource_regions(
                    alias,
                    &component_name,
                    &component.resource_regions,
                    &thing_names,
                ),
                resource_layouts: namespace_system_resource_layouts(
                    alias,
                    &component_name,
                    &component.resource_layouts,
                    &thing_names,
                ),
                resource_allocations: namespace_system_resource_allocations(
                    alias,
                    &component_name,
                    &component.resource_allocations,
                    &thing_names,
                ),
                lock_guards: namespace_system_lock_guards(
                    alias,
                    &component_name,
                    &component.lock_guards,
                    &thing_names,
                ),
                execution_contexts: namespace_system_execution_contexts(
                    &component_name,
                    &component.execution_contexts,
                ),
                interrupt_priorities: namespace_system_interrupt_priorities(
                    &component_name,
                    &component.interrupt_priorities,
                ),
                interrupt_masks: namespace_system_interrupt_masks(
                    &component_name,
                    &component.interrupt_masks,
                ),
                scheduler_tasks: namespace_system_scheduler_tasks(
                    &component_name,
                    &component.scheduler_tasks,
                ),
                scheduler_task_priorities: namespace_system_scheduler_task_priorities(
                    &component_name,
                    &component.scheduler_task_priorities,
                ),
                scheduler_task_timings: namespace_system_scheduler_task_timings(
                    &component_name,
                    &component.scheduler_task_timings,
                ),
                capabilities: component
                    .capabilities
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                effects: component
                    .effects
                    .iter()
                    .map(|text| qualify_reference_text(text, alias, &thing_names))
                    .collect(),
                guarantees: component.guarantees.clone(),
                traces: component
                    .traces
                    .iter()
                    .map(|trace| qualify_name(alias, trace))
                    .collect(),
                provenance: format!("system_component:{component_name}"),
            },
        );
    }

    for failure in document.failures.values() {
        let failure_name = qualify_name(alias, &failure.name);
        namespaced.failures.insert(
            failure_name.clone(),
            AilFailure {
                name: failure_name.clone(),
                condition: qualify_reference_text(&failure.condition, alias, &thing_names),
                handling: failure.handling.clone(),
                traces: failure
                    .traces
                    .iter()
                    .map(|trace| qualify_name(alias, trace))
                    .collect(),
                provenance: format!("failure:{failure_name}"),
            },
        );
    }
    namespaced
}

fn namespace_system_resources(
    alias: &str,
    component_name: &str,
    resources: &BTreeMap<String, AilSystemResource>,
    thing_names: &[String],
) -> BTreeMap<String, AilSystemResource> {
    resources
        .values()
        .map(|resource| {
            (
                resource.name.clone(),
                AilSystemResource {
                    name: resource.name.clone(),
                    type_name: qualify_type_name(&resource.type_name, alias, thing_names),
                    provenance: format!(
                        "system_component:{component_name}.resource:{}",
                        resource.name
                    ),
                },
            )
        })
        .collect()
}

fn namespace_system_resource_regions(
    alias: &str,
    component_name: &str,
    resource_regions: &[AilSystemResourceRegion],
    thing_names: &[String],
) -> Vec<AilSystemResourceRegion> {
    resource_regions
        .iter()
        .map(|placement| AilSystemResourceRegion {
            resource_name: qualify_reference_text(&placement.resource_name, alias, thing_names),
            region_name: qualify_name(alias, &placement.region_name),
            provenance: format!(
                "system_component:{component_name}.region:{}",
                placement.region_name
            ),
        })
        .collect()
}

fn namespace_system_resource_layouts(
    alias: &str,
    component_name: &str,
    resource_layouts: &[AilSystemResourceLayout],
    thing_names: &[String],
) -> Vec<AilSystemResourceLayout> {
    resource_layouts
        .iter()
        .map(|layout| AilSystemResourceLayout {
            resource_name: qualify_reference_text(&layout.resource_name, alias, thing_names),
            layout: layout.layout.clone(),
            provenance: format!(
                "system_component:{component_name}.layout:{}",
                layout.resource_name
            ),
        })
        .collect()
}

fn namespace_system_resource_allocations(
    alias: &str,
    component_name: &str,
    resource_allocations: &[AilSystemResourceAllocation],
    thing_names: &[String],
) -> Vec<AilSystemResourceAllocation> {
    resource_allocations
        .iter()
        .map(|allocation| AilSystemResourceAllocation {
            resource_name: qualify_reference_text(&allocation.resource_name, alias, thing_names),
            placement: allocation.placement.clone(),
            provenance: format!(
                "system_component:{component_name}.allocation:{}",
                allocation.resource_name
            ),
        })
        .collect()
}

fn namespace_system_lock_guards(
    alias: &str,
    component_name: &str,
    lock_guards: &[AilSystemLockGuard],
    thing_names: &[String],
) -> Vec<AilSystemLockGuard> {
    lock_guards
        .iter()
        .map(|guard| AilSystemLockGuard {
            resource_name: qualify_reference_text(&guard.resource_name, alias, thing_names),
            lock_name: qualify_reference_text(&guard.lock_name, alias, thing_names),
            provenance: format!(
                "system_component:{component_name}.lock_guard:{}",
                guard.resource_name
            ),
        })
        .collect()
}

fn namespace_system_execution_contexts(
    component_name: &str,
    execution_contexts: &[AilSystemExecutionContext],
) -> Vec<AilSystemExecutionContext> {
    execution_contexts
        .iter()
        .map(|context| AilSystemExecutionContext {
            name: context.name.clone(),
            provenance: format!("system_component:{component_name}.context:{}", context.name),
        })
        .collect()
}

fn namespace_system_interrupt_priorities(
    component_name: &str,
    interrupt_priorities: &[AilSystemInterruptPriority],
) -> Vec<AilSystemInterruptPriority> {
    interrupt_priorities
        .iter()
        .map(|priority| AilSystemInterruptPriority {
            context_name: priority.context_name.clone(),
            priority: priority.priority.clone(),
            provenance: format!(
                "system_component:{component_name}.priority:{}",
                priority.context_name
            ),
        })
        .collect()
}

fn namespace_system_interrupt_masks(
    component_name: &str,
    interrupt_masks: &[AilSystemInterruptMask],
) -> Vec<AilSystemInterruptMask> {
    interrupt_masks
        .iter()
        .map(|mask| AilSystemInterruptMask {
            context_name: mask.context_name.clone(),
            mask: mask.mask.clone(),
            provenance: format!(
                "system_component:{component_name}.interrupt_mask:{}",
                mask.context_name
            ),
        })
        .collect()
}

fn namespace_system_scheduler_tasks(
    component_name: &str,
    scheduler_tasks: &[AilSystemSchedulerTask],
) -> Vec<AilSystemSchedulerTask> {
    scheduler_tasks
        .iter()
        .map(|task| AilSystemSchedulerTask {
            task_name: task.task_name.clone(),
            context_name: task.context_name.clone(),
            provenance: format!("system_component:{component_name}.task:{}", task.task_name),
        })
        .collect()
}

fn namespace_system_scheduler_task_priorities(
    component_name: &str,
    scheduler_task_priorities: &[AilSystemSchedulerTaskPriority],
) -> Vec<AilSystemSchedulerTaskPriority> {
    scheduler_task_priorities
        .iter()
        .map(|priority| AilSystemSchedulerTaskPriority {
            task_name: priority.task_name.clone(),
            priority: priority.priority.clone(),
            provenance: format!(
                "system_component:{component_name}.task_priority:{}",
                priority.task_name
            ),
        })
        .collect()
}

fn namespace_system_scheduler_task_timings(
    component_name: &str,
    scheduler_task_timings: &[AilSystemSchedulerTaskTiming],
) -> Vec<AilSystemSchedulerTaskTiming> {
    scheduler_task_timings
        .iter()
        .map(|timing| AilSystemSchedulerTaskTiming {
            task_name: timing.task_name.clone(),
            deadline: timing.deadline.clone(),
            budget: timing.budget.clone(),
            provenance: format!(
                "system_component:{component_name}.task_timing:{}",
                timing.task_name
            ),
        })
        .collect()
}

fn namespace_pass_values(
    alias: &str,
    pass_name: &str,
    values: &BTreeMap<String, AilPassValue>,
    thing_names: &[String],
) -> BTreeMap<String, AilPassValue> {
    values
        .values()
        .map(|value| {
            (
                value.name.clone(),
                AilPassValue {
                    name: value.name.clone(),
                    type_name: qualify_type_name(&value.type_name, alias, thing_names),
                    provenance: format!("compiler_pass:{pass_name}.value:{}", value.name),
                },
            )
        })
        .collect()
}

fn namespace_tool_slots(
    alias: &str,
    tool_name: &str,
    slots: &BTreeMap<String, AilToolSlot>,
    thing_names: &[String],
) -> BTreeMap<String, AilToolSlot> {
    slots
        .values()
        .map(|slot| {
            (
                slot.name.clone(),
                AilToolSlot {
                    name: slot.name.clone(),
                    type_name: qualify_type_name(&slot.type_name, alias, thing_names),
                    is_secret: slot.is_secret,
                    provenance: format!("tool:{tool_name}.slot:{}", slot.name),
                },
            )
        })
        .collect()
}

fn qualify_name(alias: &str, name: &str) -> String {
    if name.starts_with(&format!("{alias}.")) {
        name.to_string()
    } else {
        format!("{alias}.{name}")
    }
}

fn qualify_failure_reference(reference: &str, alias: &str, failure_names: &[String]) -> String {
    failure_names
        .iter()
        .find(|failure| failure.eq_ignore_ascii_case(reference))
        .map(|failure| qualify_name(alias, failure))
        .unwrap_or_else(|| reference.to_string())
}

fn qualify_type_name(type_name: &str, alias: &str, thing_names: &[String]) -> String {
    let type_name = normalize_type_name(type_name);
    if thing_names.iter().any(|thing| thing == &type_name) {
        return qualify_name(alias, &type_name);
    }
    for wrapper in ["Secret", "List", "Option"] {
        if let Some(inner) = generic_inner(&type_name, wrapper) {
            return format!(
                "{wrapper}<{}>",
                qualify_type_name(inner, alias, thing_names)
            );
        }
    }
    type_name
}

fn qualify_reference_text(text: &str, alias: &str, thing_names: &[String]) -> String {
    let mut qualified = text.to_string();
    for thing in thing_names {
        let lower = thing.to_ascii_lowercase();
        qualified = qualified.replace(&format!("the {lower}"), &format!("the {alias}.{thing}"));
        qualified = qualified.replace(&format!("{lower} "), &format!("{alias}.{thing} "));
    }
    qualified
}

fn build_ail_requirements_prompt(package: &AilPackage, user_prompt: &str) -> String {
    let coverage = ail_requirements_prompt_coverage(&package.metadata.profile);
    format!(
        concat!(
            "Draft AIL requirements for package {}.\n",
            "Use the {} profile and conformance level {}.\n",
            "Package features: {}.\n",
            "Output only an AIL-Requirements artifact with concise bullet points. Do not include code fences, AIL-Spec, implementation code, backend source, or reasoning.\n",
            "The first line must be exactly AIL-Requirements:. Return at least six requirement bullets, and every requirement bullet must start with \"- \"; do not use \"*\" bullets, numbered lists, tables, or Markdown emphasis.\n",
            "{}\n",
            "These requirements are an intermediate artifact. The next compiler step will transform them into AIL-Spec, then AIL-Core, then AIL-Bytecode.\n\n",
            "HUMAN REQUEST:\n",
            "{}\n"
        ),
        package.metadata.name,
        package.metadata.profile,
        package.metadata.conformance,
        package.metadata.features.join(", "),
        coverage,
        user_prompt
    )
}

fn ail_requirements_prompt_coverage(profile: &str) -> &'static str {
    match profile {
        "AgentTool" => {
            "Requirements must name the tool capability, tool inputs and outputs, external calls, failure cases, guarantees, traces, secrets, permissions, and approvals that the final checked AIL tool must preserve."
        }
        "Compiler" => {
            "Requirements must name the compiler pass, IR inputs and outputs, transformations, failure cases, guarantees, traces, and bytecode boundaries that the final checked AIL compiler pass must preserve."
        }
        "System" => {
            "Requirements must name system components, resources, capabilities, effects, ownership or borrowing rules, scheduler/interrupt/lock constraints, guarantees, traces, and runtime inputs that the final checked AIL system component must preserve."
        }
        _ => {
            "Requirements must name application domain objects, required fields, actions, failure cases, guarantees, traces, secrets, permissions, and runtime inputs that the final checked AIL application must preserve."
        }
    }
}

fn build_ail_requirements_repair_prompt(
    package: &AilPackage,
    user_prompt: &str,
    previous_requirements_text: &str,
    diagnostics: &[AilDiagnostic],
) -> String {
    let diagnostics_text = diagnostics
        .iter()
        .map(AilDiagnostic::detailed_message)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        concat!(
            "Repair AIL requirements for package {}.\n",
            "Use the {} profile and conformance level {}.\n",
            "Return only an AIL-Requirements artifact with concise bullet points. Do not include code fences, AIL-Spec, implementation code, backend source, or reasoning.\n",
            "The first line must be exactly AIL-Requirements:. Return at least six requirement bullets, and every requirement bullet must start with \"- \"; do not use \"*\" bullets, numbered lists, tables, or Markdown emphasis.\n",
            "The repaired requirements must be sufficient for the next toolchain step to draft checked AIL-Spec, lower it to AIL-Core, and compile AIL-Bytecode.\n\n",
            "ORIGINAL HUMAN REQUEST:\n",
            "{}\n\n",
            "PREVIOUS AIL-REQUIREMENTS:\n",
            "{}\n\n",
            "REQUIREMENTS DIAGNOSTICS:\n",
            "{}\n"
        ),
        package.metadata.name,
        package.metadata.profile,
        package.metadata.conformance,
        user_prompt,
        previous_requirements_text,
        diagnostics_text
    )
}

struct AilRequirementsTopic {
    code: &'static str,
    message: &'static str,
    terms: &'static [&'static str],
    repair: &'static str,
}

fn required_requirements_topics(profile: &str) -> Vec<AilRequirementsTopic> {
    match profile {
        "AgentTool" => vec![
            AilRequirementsTopic {
                code: "AILR006",
                message: "requirements are missing tool surface coverage",
                terms: &["tool", "agent may request"],
                repair: "Name the tool capability the AI Agent may request.",
            },
            AilRequirementsTopic {
                code: "AILR008",
                message: "requirements are missing input coverage",
                terms: &["input", "needs", "requires"],
                repair: "Name the inputs the tool needs before execution.",
            },
            AilRequirementsTopic {
                code: "AILR009",
                message: "requirements are missing output coverage",
                terms: &["output", "outputs", "produces", "returns"],
                repair: "Name the outputs the tool produces.",
            },
            AilRequirementsTopic {
                code: "AILR010",
                message: "requirements are missing permission or approval coverage",
                terms: &["permission", "approval", "approver", "may request"],
                repair: "Name the permission or approval rules required to run the tool.",
            },
            requirement_failure_topic(),
            requirement_guarantee_topic(),
            requirement_trace_topic(),
        ],
        "Compiler" => vec![
            AilRequirementsTopic {
                code: "AILR006",
                message: "requirements are missing compiler pass coverage",
                terms: &["compiler pass", "pass"],
                repair: "Name the compiler pass and the graph transformation it performs.",
            },
            AilRequirementsTopic {
                code: "AILR008",
                message: "requirements are missing compiler input coverage",
                terms: &["input", "needs", "reads"],
                repair: "Name the values or graph inputs the compiler pass reads.",
            },
            AilRequirementsTopic {
                code: "AILR009",
                message: "requirements are missing compiler output coverage",
                terms: &["output", "produces", "adds", "writes"],
                repair: "Name the values or graph outputs the compiler pass writes.",
            },
            requirement_failure_topic(),
            requirement_guarantee_topic(),
            requirement_trace_topic(),
        ],
        "System" => vec![
            AilRequirementsTopic {
                code: "AILR006",
                message: "requirements are missing system component coverage",
                terms: &["system component", "component"],
                repair: "Name the system component being compiled.",
            },
            AilRequirementsTopic {
                code: "AILR008",
                message: "requirements are missing resource coverage",
                terms: &["resource", "buffer", "device", "lock", "region"],
                repair: "Name the resources, locks, regions, or devices the component uses.",
            },
            AilRequirementsTopic {
                code: "AILR009",
                message: "requirements are missing capability or effect coverage",
                terms: &["capability", "effect", "performs", "read", "write"],
                repair: "Name the capabilities and effects the system component requires.",
            },
            requirement_failure_topic(),
            requirement_guarantee_topic(),
            requirement_trace_topic(),
        ],
        _ => vec![
            AilRequirementsTopic {
                code: "AILR006",
                message: "requirements are missing domain data coverage",
                terms: &[
                    "field", "fields", "data", "object", "objects", "thing", "record",
                ],
                repair: "Name the domain objects and important fields the application stores.",
            },
            AilRequirementsTopic {
                code: "AILR007",
                message: "requirements are missing action coverage",
                terms: &["action", "behavior", "when", "create", "close", "update"],
                repair: "Name the application actions or behaviors that must compile.",
            },
            AilRequirementsTopic {
                code: "AILR008",
                message: "requirements are missing runtime input or precondition coverage",
                terms: &["input", "requires", "precondition", "must", "needs"],
                repair: "Name the runtime inputs or preconditions required by each action.",
            },
            requirement_failure_topic(),
            requirement_guarantee_topic(),
            requirement_trace_topic(),
        ],
    }
}

fn requirement_failure_topic() -> AilRequirementsTopic {
    AilRequirementsTopic {
        code: "AILR003",
        message: "requirements are missing failure coverage",
        terms: &[
            "failure",
            "fails",
            "error",
            "denied",
            "not found",
            "missing",
        ],
        repair: "Name at least one expected failure case or explicitly state what cannot fail.",
    }
}

fn requirement_trace_topic() -> AilRequirementsTopic {
    AilRequirementsTopic {
        code: "AILR004",
        message: "requirements are missing trace coverage",
        terms: &["trace", "traces", "records", "audit"],
        repair: "Name trace or audit events the compiled bytecode must emit.",
    }
}

fn requirement_guarantee_topic() -> AilRequirementsTopic {
    AilRequirementsTopic {
        code: "AILR005",
        message: "requirements are missing guarantee coverage",
        terms: &[
            "guarantee",
            "guarantees",
            "always",
            "must preserve",
            "does not",
        ],
        repair: "Name guarantees that must hold after execution.",
    }
}

fn requirements_mentions_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

fn build_ail_repair_prompt(
    package: &AilPackage,
    user_prompt: &str,
    requirements_text: &str,
    previous_spec_text: &str,
    diagnostics: &[AilDiagnostic],
) -> String {
    let diagnostics_text = diagnostics
        .iter()
        .map(AilDiagnostic::detailed_message)
        .collect::<Vec<_>>()
        .join("\n");
    let repair_request = format!(
        concat!(
            "Repair an AIL-Spec candidate for package {}.\n",
            "Keep the original human request and requirements, but fix every checker diagnostic before returning the candidate.\n",
            "Do not explain the fix. Do not generate host-language source. The repaired candidate will be parsed, checked, lowered to AIL-Core, and compiled to AIL-Bytecode.\n\n",
            "ORIGINAL HUMAN REQUEST:\n",
            "{}\n\n",
            "DRAFT REQUIREMENTS:\n",
            "{}\n\n",
            "PREVIOUS AIL-SPEC CANDIDATE:\n",
            "{}\n\n",
            "CHECKER DIAGNOSTICS:\n",
            "{}\n"
        ),
        package.metadata.name, user_prompt, requirements_text, previous_spec_text, diagnostics_text
    );
    build_ail_draft_prompt(package, &repair_request)
}

fn build_ail_draft_prompt(package: &AilPackage, user_prompt: &str) -> String {
    let surface_shape = if package.metadata.profile == "System"
        || package
            .metadata
            .features
            .iter()
            .any(|feature| feature == "system-components")
    {
        concat!(
            "Use this exact System profile surface shape:\n",
            "System component: <human label>.\n\n",
            "The component uses:\n\n",
            "- <resource>: <Type>\n\n",
            "The component owns:\n\n",
            "- <resource>\n\n",
            "The component borrows:\n\n",
            "- <resource>\n\n",
            "The component mutably borrows:\n\n",
            "- <resource>\n\n",
            "The component places:\n\n",
            "- <resource> in <region>\n\n",
            "The component lays out:\n\n",
            "- <resource>: <layout rule>\n\n",
            "The component allocates:\n\n",
            "- <resource>: <placement>\n\n",
            "The component guards:\n\n",
            "- <resource> with <lock resource>\n\n",
            "The component runs in context:\n\n",
            "- <context>\n\n",
            "The component sets interrupt priority:\n\n",
            "- <context>: <priority>\n\n",
            "The component masks interrupt:\n\n",
            "- <context>: <mask rule>\n\n",
            "The component schedules task:\n\n",
            "- <task>: <context>\n\n",
            "The component sets task priority:\n\n",
            "- <task>: <priority>\n\n",
            "The component sets task timing:\n\n",
            "- <task>: deadline <duration>, budget <duration>\n\n",
            "The component requires capability:\n\n",
            "- <capability rule>\n\n",
            "The component performs:\n\n",
            "- <effect>\n\n",
            "The component records:\n\n",
            "- <TraceName>\n\n",
            "The component guarantees:\n\n",
            "- <guarantee>\n"
        )
    } else if package.metadata.profile == "Compiler"
        || package
            .metadata
            .features
            .iter()
            .any(|feature| feature == "compiler-passes")
    {
        concat!(
            "Use this exact Compiler profile surface shape:\n",
            "Compiler pass: <human label>.\n\n",
            "The pass describes what compiler artifact it reads, writes, or validates.\n\n",
            "The pass needs:\n\n",
            "- <input>: <Type>\n\n",
            "The pass produces:\n\n",
            "- <output>: <Type>\n\n",
            "When the compiler runs <human label>:\n\n",
            "- the system reads <compiler artifact or value>\n",
            "- the system finds <semantic item>\n",
            "- the system checks <rule>\n",
            "- the system adds <candidate artifact>\n",
            "- the system emits <diagnostic>\n",
            "- the system guarantees <guarantee>\n",
            "- the system records a trace event named <TraceName>\n\n",
            "Failure <Name> happens when <condition>:\n\n",
            "- <handling rule>\n",
            "- the trace records <TraceName>\n"
        )
    } else if package.metadata.profile == "AgentTool"
        || package
            .metadata
            .features
            .iter()
            .any(|feature| feature == "tools")
    {
        concat!(
            "Use this exact AgentTool surface shape:\n",
            "Tool: <human label>.\n\n",
            "The AI Agent may request <human label> when:\n\n",
            "- <rule>\n\n",
            "The tool needs:\n\n",
            "- <input>: <Type>\n\n",
            "The tool produces:\n\n",
            "- <output>: <Type>\n\n",
            "The tool can:\n\n",
            "- read <resource>\n",
            "- call <external capability>\n",
            "- write <effect>\n",
            "- create <effect>\n\n",
            "The tool must not:\n\n",
            "- reveal <secret input>\n\n",
            "The tool requires permission:\n\n",
            "- <permission rule>\n\n",
            "The tool requires approval:\n\n",
            "- <approval rule>\n\n",
            "The tool records:\n\n",
            "- <TraceName>\n\n",
            "The tool guarantees:\n\n",
            "- <guarantee>\n\n",
            "Failure <Name> happens when <condition>:\n\n",
            "- <handling rule>\n",
            "- the trace records <TraceName>\n"
        )
    } else {
        concat!(
            "Use this exact surface shape:\n",
            "The application <Name> manages <purpose>.\n\n",
            "A <Thing> has:\n\n",
            "- <field>: <Type>\n\n",
            "Action: <human label>.\n\n",
            "When <trigger>:\n\n",
            "- the system requires <rule>\n",
            "- the system reads <field or effect>\n",
            "- the system changes <field or effect>\n",
            "- the system does not reveal <secret field> to <audience>\n",
            "- the system guarantees <guarantee>\n",
            "- the system records a trace event named <TraceName>\n\n",
            "Failure <Name> happens when <condition>:\n\n",
            "- <handling rule>\n",
            "- the trace records <TraceName>\n"
        )
    };
    format!(
        concat!(
            "Draft an AIL-Spec candidate for package {}.\n",
            "Use the {} profile and conformance level {}.\n",
            "Package features: {}.\n",
            "Output only parseable AIL-Spec structured English. Do not include code fences, Markdown commentary, labels like Application:, or reasoning.\n",
            "The checker will decide whether the candidate is accepted, so preserve explicit things, fields, tools, actions, system components, capabilities, failures, guarantees, traces, and secret handling.\n\n",
            "Use canonical AIL type spellings: Text, State<Open, Closed>, List<Text>, Option<Text>, and Secret<List<Text>> for a secret list of text values.\n\n",
            "{}\n\n",
            "HUMAN REQUEST:\n",
            "{}\n"
        ),
        package.metadata.name,
        package.metadata.profile,
        package.metadata.conformance,
        package.metadata.features.join(", "),
        surface_shape,
        user_prompt
    )
}

fn failed_run(
    document: &AilDocument,
    final_state: BTreeMap<String, String>,
    mut trace: Vec<String>,
    failure_name: &str,
) -> AilRunResult {
    trace.push(format!("failure {failure_name}"));
    if let Some(failure) = document.failures.get(failure_name) {
        for event in &failure.traces {
            trace.push(format!("trace {event}"));
        }
    }
    AilRunResult {
        status: "failed".to_string(),
        failure: Some(failure_name.to_string()),
        final_state,
        trace,
    }
}

fn input_requirement_keys(document: &AilDocument, requirement: &str) -> Option<Vec<String>> {
    let text = normalized_field_reference_text(requirement);
    let normalized = text.to_ascii_lowercase();
    if normalized.contains(" to ")
        || normalized.contains(" is ")
        || normalized.contains(" has role ")
        || normalized.contains(" has permission to ")
        || (normalized.contains(" has ") && normalized.ends_with(" role"))
    {
        return None;
    }

    let parts = normalized
        .split(" and ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }

    let mut keys = Vec::new();
    for part in parts {
        let key = application_user_field_key(document, part)
            .or_else(|| referenced_runtime_field_key(document, part))?;
        keys.push(key);
    }
    keys.sort();
    keys.dedup();
    (!keys.is_empty()).then_some(keys)
}

fn application_user_field_key(document: &AilDocument, text: &str) -> Option<String> {
    let normalized = normalized_field_reference_text(text).to_ascii_lowercase();
    for user in &document.application.users {
        let user_key = runtime_subject_key(user);
        let Some(field_text) = normalized.strip_prefix(&format!("{user_key} ")) else {
            continue;
        };
        let field_name = user_field_name(document, field_text.trim())?;
        return Some(format!("{user_key}.{field_name}"));
    }
    None
}

fn user_field_name(document: &AilDocument, text: &str) -> Option<String> {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    document
        .things
        .values()
        .filter(|thing| {
            thing.name.eq_ignore_ascii_case("User") || thing.name.rsplit('.').next() == Some("User")
        })
        .flat_map(|thing| thing.fields.values())
        .find(|field| {
            field.name.to_ascii_lowercase() == normalized
                || runtime_subject_key(&field.name) == normalized
        })
        .map(|field| field.name.clone())
}

fn has_role_requirement(
    document: &AilDocument,
    requirement: &str,
) -> Option<(String, Vec<String>)> {
    let (subject_text, allowed_text) = requirement
        .rsplit_once(" has role ")
        .or_else(|| trailing_role_requirement_parts(requirement))?;
    let key = role_requirement_runtime_key(document, subject_text)?;
    let allowed_values = split_allowed_requirement_values(allowed_text);
    (!allowed_values.is_empty()).then_some((key, allowed_values))
}

fn trailing_role_requirement_parts(requirement: &str) -> Option<(&str, &str)> {
    let (subject_text, allowed_text) = requirement.rsplit_once(" has ")?;
    let allowed_text = allowed_text.trim().strip_suffix(" role")?.trim();
    (!allowed_text.is_empty()).then_some((subject_text, allowed_text))
}

fn role_requirement_runtime_key(document: &AilDocument, subject_text: &str) -> Option<String> {
    let normalized = normalized_field_reference_text(subject_text).to_ascii_lowercase();
    match normalized.as_str() {
        "actor" | "caller" | "requester" | "current user" => {
            Some(format!("{}.role", runtime_subject_key(normalized.as_str())))
        }
        _ => referenced_runtime_field_key(document, &format!("{subject_text} role"))
            .or_else(|| Some(format!("{}.role", runtime_subject_key(subject_text)))),
    }
}

fn has_permission_requirement(requirement: &str) -> Option<(String, Vec<String>)> {
    let (subject_text, allowed_text) = requirement.rsplit_once(" has permission to ")?;
    let key = format!("{}.permission", runtime_subject_key(subject_text));
    let allowed_values = split_allowed_requirement_values(allowed_text);
    (!allowed_values.is_empty()).then_some((key, allowed_values))
}

fn negative_field_requirement(
    document: &AilDocument,
    requirement: &str,
) -> Option<(String, String)> {
    let (field_text, forbidden) = requirement
        .split_once(" not to be ")
        .or_else(|| requirement.split_once(" is not "))?;
    let key = referenced_runtime_field_key(document, field_text)?;
    let forbidden = forbidden
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.')
        .next()
        .unwrap_or("")
        .trim();
    (!forbidden.is_empty()).then(|| (key, forbidden.to_string()))
}

fn positive_field_requirement(
    document: &AilDocument,
    requirement: &str,
) -> Option<(String, Vec<String>)> {
    if requirement.contains(" not to be ") || requirement.contains(" is not ") {
        return None;
    }
    let (field_text, allowed_text) = requirement
        .rsplit_once(" to be ")
        .or_else(|| requirement.rsplit_once(" is "))?;
    let key = referenced_runtime_field_key(document, field_text)?;
    let allowed_values = split_allowed_requirement_values(allowed_text);
    (!allowed_values.is_empty()).then_some((key, allowed_values))
}

fn field_after_requirement(document: &AilDocument, requirement: &str) -> Option<(String, String)> {
    let marker = " to be later than ";
    let (source_text, key_text) = requirement.split_once(marker)?;
    let source = current_time_runtime_key(source_text)
        .or_else(|| referenced_runtime_field_key(document, source_text))?;
    let key = referenced_runtime_field_key(document, key_text)?;
    Some((source, key))
}

fn current_time_runtime_key(text: &str) -> Option<String> {
    let normalized = normalized_field_reference_text(text).to_ascii_lowercase();
    (normalized == "current time").then(|| "current.time".to_string())
}

fn field_copy_assignment(document: &AilDocument, write: &str) -> Option<(String, String)> {
    let (source_text, destination_text) = write.split_once(" as ")?;
    let source_key = application_user_id_key(document, source_text)
        .or_else(|| referenced_runtime_field_key(document, source_text))?;
    let destination_key = copied_destination_key(document, destination_text, &source_key)?;
    Some((source_key, destination_key))
}

fn application_user_id_key(document: &AilDocument, text: &str) -> Option<String> {
    let normalized = normalized_field_reference_text(text).to_ascii_lowercase();
    for user in &document.application.users {
        let user_key = runtime_subject_key(user);
        if normalized == user_key && user_field_name(document, "id").is_some() {
            return Some(format!("{user_key}.id"));
        }
    }
    None
}

fn copied_destination_key(
    document: &AilDocument,
    destination_text: &str,
    source_key: &str,
) -> Option<String> {
    let destination_key = referenced_runtime_field_key(document, destination_text)?;
    if source_key.ends_with(".id")
        && runtime_field_type_name(document, &destination_key)
            .and_then(|type_name| referenced_thing_type(document, type_name))
            .is_some_and(|thing| thing.fields.contains_key("id"))
    {
        return Some(format!("{destination_key}.id"));
    }
    Some(destination_key)
}

fn runtime_field_type_name<'a>(document: &'a AilDocument, key: &str) -> Option<&'a str> {
    for thing in document.things.values() {
        for field in thing.fields.values() {
            if key == runtime_field_key(&thing.name, &field.name) {
                return Some(&field.type_name);
            }
        }
    }
    None
}

fn field_write_assignment(document: &AilDocument, write: &str) -> Option<(String, String)> {
    field_write_to_assignment(document, write)
        .or_else(|| field_write_with_assignment(document, write))
}

fn field_write_to_assignment(document: &AilDocument, write: &str) -> Option<(String, String)> {
    let marker = " to ";
    let (field_text, value) = write.rsplit_once(marker)?;
    let key = referenced_runtime_field_key(document, field_text)?;
    write_assignment_value(value).map(|value| (key, value))
}

fn field_write_with_assignment(document: &AilDocument, write: &str) -> Option<(String, String)> {
    let normalized = normalized_field_reference_text(write);
    let (subject_text, rest_text) = normalized.split_once(" with ")?;
    let subject_text = subject_text.to_ascii_lowercase();
    let rest_text = normalized_field_reference_text(rest_text);
    let rest_lower = rest_text.to_ascii_lowercase();
    let thing = document.things.values().find(|thing| {
        let thing_name = thing.name.to_ascii_lowercase();
        let local_name = thing.name.rsplit('.').next().unwrap_or(&thing.name);
        let local_name = local_name.to_ascii_lowercase();
        subject_text == thing_name
            || subject_text == local_name
            || subject_text.ends_with(&format!(" {thing_name}"))
            || subject_text.ends_with(&format!(" {local_name}"))
    })?;

    let mut matches = Vec::new();
    for field in thing.fields.values() {
        let field_text = field.name.to_ascii_lowercase();
        let prefix = format!("{field_text} ");
        let Some(_) = rest_lower.strip_prefix(&prefix) else {
            continue;
        };
        let value_text = rest_text.get(field.name.len()..)?.trim();
        let value = write_assignment_value(value_text)?;
        matches.push((
            field_text.len(),
            runtime_field_key(&thing.name, &field.name),
            value,
        ));
    }
    matches.sort_by_key(|(len, _, _)| std::cmp::Reverse(*len));
    matches
        .into_iter()
        .next()
        .map(|(_, key, value)| (key, value))
}

fn write_assignment_value(text: &str) -> Option<String> {
    let value = text
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.')
        .next()
        .unwrap_or("")
        .trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn failed_requirement_name(document: &AilDocument, requirement: &str, key: &str) -> String {
    let normalized = requirement.to_ascii_lowercase();
    if (normalized.contains("permission") || normalized.contains("role") || key.ends_with(".role"))
        && let Some(permission_denied) = document
            .failures
            .keys()
            .find(|name| name.rsplit('.').next() == Some("PermissionDenied"))
    {
        return permission_denied.clone();
    }
    "RequirementFailed".to_string()
}

fn split_allowed_requirement_values(text: &str) -> Vec<String> {
    text.split(',')
        .flat_map(|part| part.split(" or "))
        .map(|value| {
            value
                .trim()
                .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.')
                .to_string()
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn referenced_runtime_field_key(document: &AilDocument, text: &str) -> Option<String> {
    let normalized = text.to_ascii_lowercase();
    let mut nested_matches = Vec::new();
    let mut qualified_matches = Vec::new();
    let mut field_matches = Vec::new();
    for thing in document.things.values() {
        for field in thing.fields.values() {
            let field_text = field.name.to_ascii_lowercase();
            let thing_text = thing.name.to_ascii_lowercase();
            let qualified = format!("{thing_text} {field_text}");
            let key = runtime_field_key(&thing.name, &field.name);
            if normalized.contains(&qualified) {
                qualified_matches.push((qualified.len(), key.clone()));
            } else if normalized.contains(&field_text) {
                field_matches.push(key.clone());
            }
            if let Some(target_thing) = referenced_thing_type(document, &field.type_name) {
                for nested_field in target_thing.fields.values() {
                    let nested_field_text = nested_field.name.to_ascii_lowercase();
                    let nested_field_phrase = format!("{field_text} {nested_field_text}");
                    let qualified_nested_field_phrase =
                        format!("{thing_text} {nested_field_phrase}");
                    let nested_key = format!("{key}.{}", runtime_subject_key(&nested_field.name));
                    if normalized.contains(&qualified_nested_field_phrase) {
                        nested_matches.push((qualified_nested_field_phrase.len(), nested_key));
                    } else if normalized.contains(&nested_field_phrase) {
                        nested_matches.push((nested_field_phrase.len(), nested_key));
                    }
                }
            }
        }
    }
    nested_matches.sort_by_key(|(len, _)| std::cmp::Reverse(*len));
    if let Some((_, key)) = nested_matches.into_iter().next() {
        return Some(key);
    }
    qualified_matches.sort_by_key(|(len, _)| std::cmp::Reverse(*len));
    if let Some((_, key)) = qualified_matches.into_iter().next() {
        return Some(key);
    }
    field_matches.sort();
    field_matches.dedup();
    (field_matches.len() == 1).then(|| field_matches.remove(0))
}

fn referenced_thing_type<'a>(document: &'a AilDocument, type_name: &str) -> Option<&'a AilThing> {
    let normalized = normalize_type_name(type_name);
    let unwrapped = unwrap_ail_value_type(&normalized);
    document
        .things
        .values()
        .find(|thing| thing.name == unwrapped || thing.name.rsplit('.').next() == Some(unwrapped))
}

fn unwrap_ail_value_type(type_name: &str) -> &str {
    let mut current = type_name;
    while let Some(inner) = generic_inner(current, "Option")
        .or_else(|| generic_inner(current, "List"))
        .or_else(|| generic_inner(current, "Secret"))
    {
        current = inner;
    }
    current
}

fn runtime_field_key(thing_name: &str, field_name: &str) -> String {
    format!("{}.{}", runtime_subject_key(thing_name), field_name)
}

fn is_secret_runtime_state_key(document: &AilDocument, key: &str) -> bool {
    if document.things.values().any(|thing| {
        thing
            .fields
            .values()
            .any(|field| field.is_secret && key == runtime_field_key(&thing.name, &field.name))
    }) {
        return true;
    }
    document.tools.values().any(|tool| {
        tool.inputs
            .values()
            .chain(tool.outputs.values())
            .any(|slot| {
                slot.is_secret
                    && (key == slot.name || key == format!("{}.{}", tool.name, slot.name))
            })
    })
}

fn runtime_subject_key(subject: &str) -> String {
    subject
        .trim()
        .trim_start_matches("the ")
        .trim()
        .to_ascii_lowercase()
}

fn trim_trailing_blank_lines(lines: &mut Vec<String>) {
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
}

fn required_metadata(values: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    values
        .get(key)
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or_else(|| format!("AIL package metadata missing '{key}'"))
}

fn file_name_or_display(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn parse_ail_patch_field(line: &str) -> Option<AilPatchChange> {
    let field_spec = line.strip_prefix("add field ")?;
    let (path, type_name) = field_spec.split_once(':')?;
    let (thing_name, field_name) = path.trim().split_once('.')?;
    Some(AilPatchChange::AddField {
        thing_name: thing_name.trim().to_string(),
        field_name: field_name.trim().to_string(),
        type_name: type_name.trim().to_string(),
    })
}

fn apply_ail_patch_action_line(action: &mut AilAction, line: &str) {
    if let Some(trigger) = line
        .strip_prefix("when ")
        .map(|trigger| trigger.trim().trim_end_matches(':').to_string())
    {
        action.trigger = trigger;
    } else if let Some(requirement) = line.strip_prefix("requires ") {
        action.requirements.push(trim_sentence(requirement));
    } else if let Some(read) = line.strip_prefix("reads ") {
        action.reads.push(trim_sentence(read));
    } else if let Some(write) = line.strip_prefix("changes ") {
        action.writes.push(trim_sentence(write));
    } else if let Some(write) = line.strip_prefix("creates ") {
        action.writes.push(trim_sentence(write));
    } else if let Some(protection) = line.strip_prefix("does not reveal ") {
        action.secret_protections.push(trim_sentence(protection));
    } else if let Some(guarantee) = line.strip_prefix("guarantees ") {
        action.guarantees.push(trim_sentence(guarantee));
    } else if let Some(trace) = line.strip_prefix("records trace ") {
        action.traces.push(trim_sentence(trace));
    } else if let Some(trace) = line.strip_prefix("records a trace event named ") {
        action.traces.push(trim_sentence(trace));
    } else if let Some(failure) = line.strip_prefix("if ") {
        action.failures.push(trim_sentence(failure));
    }
}

fn check_requirement_reference_diagnostics(core: &AilCore) -> Vec<AilDiagnostic> {
    let known_subjects = core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Thing")
        .map(|node| node.name.to_ascii_lowercase())
        .collect::<std::collections::BTreeSet<_>>();
    let field_names = core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Field")
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "requires")
    {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        let Some(rule) = node_by_id.get(&edge.target) else {
            continue;
        };
        let Some(reference) = existence_requirement_reference(&rule.name) else {
            continue;
        };
        if !known_subjects.contains(&reference.to_ascii_lowercase()) {
            if referenced_core_field_name(&field_names, &reference).is_some() {
                continue;
            }
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL001",
                    format!(
                        "unknown requirement reference '{reference}' in action {}",
                        action.name
                    ),
                )
                .with_source_provenance(node_provenance(core, &rule.id))
                .with_affected_graph_item(format!("node:{}", rule.id))
                .with_repair_suggestion(format!(
                    "Declare a Thing named '{reference}' or update the requirement to reference an existing thing."
                )),
            );
        }
    }
    diagnostics
}

fn check_requirement_field_references(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let thing_names = core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Thing")
        .map(|node| node.name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let field_names = core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Field")
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    let mut diagnostics = Vec::new();

    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "requires")
    {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        let Some(rule) = node_by_id.get(&edge.target) else {
            continue;
        };
        let Some(field_text) = requirement_field_reference_text(&rule.name) else {
            continue;
        };
        if !looks_like_field_reference(&field_text, &thing_names) {
            continue;
        }
        if referenced_core_field_name(&field_names, &field_text).is_none() {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL007",
                    format!(
                        "action {} requirement references unknown field '{}'",
                        action.name, field_text
                    ),
                )
                .with_source_provenance(node_provenance(core, &rule.id))
                .with_affected_graph_item(format!("node:{}", rule.id))
                .with_repair_suggestion(format!(
                    "Declare field '{field_text}' on the referenced thing or update the requirement to an existing field."
                )),
            );
        }
    }
    diagnostics
}

fn check_field_types(core: &AilCore) -> Vec<AilDiagnostic> {
    let declared_types = core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Thing")
        .map(|node| node.name.as_str())
        .collect::<BTreeSet<_>>();
    core.graph
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind.as_str(),
                "Field" | "Input" | "Output" | "Value" | "Resource"
            )
        })
        .filter_map(|node| {
            let type_name = node.type_name.as_deref()?;
            (!is_known_ail_type(type_name, &declared_types)).then(|| {
                let kind = typed_node_diagnostic_kind(&node.kind);
                AilDiagnostic::error(
                    "AIL006",
                    format!("{} {} has unknown type '{}'", kind, node.name, type_name),
                )
                .with_source_provenance(node_provenance(core, &node.id))
                .with_affected_graph_item(format!("node:{}", node.id))
                .with_repair_suggestion(format!(
                    "Use a supported AIL type for {} {} or declare a Thing named '{}'.",
                    kind,
                    node.name,
                    suggested_declared_type_name(type_name)
                ))
            })
        })
        .collect()
}

fn typed_node_diagnostic_kind(kind: &str) -> &'static str {
    match kind {
        "Input" => "input",
        "Output" => "output",
        "Value" => "value",
        "Resource" => "resource",
        _ => "field",
    }
}

fn suggested_declared_type_name(type_name: &str) -> String {
    for wrapper in ["Option", "List", "Secret"] {
        if let Some(inner) = generic_inner(type_name, wrapper) {
            return suggested_declared_type_name(inner);
        }
    }
    type_name.to_string()
}

fn check_action_failure_declarations(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "may_fail_with")
    {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        let Some(failure) = node_by_id.get(&edge.target) else {
            continue;
        };
        if failure
            .attributes
            .get("declared")
            .is_none_or(|value| value != "true")
        {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL003",
                    format!(
                        "action {} names failure '{}' without a declared Failure section",
                        action.name, failure.name
                    ),
                )
                .with_source_provenance(
                    edge.attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &failure.id)),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id))
                .with_repair_suggestion(format!(
                    "Add a 'Failure {} happens when ...' section with handling and trace bullets.",
                    failure.name
                )),
            );
        }
    }
    diagnostics
}

fn check_secret_write_protection(core: &AilCore) -> Vec<AilDiagnostic> {
    check_secret_access_protection(
        core,
        "writes",
        "AIL002",
        "written without an explicit protection rule",
    )
}

fn check_secret_read_protection(core: &AilCore) -> Vec<AilDiagnostic> {
    check_secret_access_protection(
        core,
        "reads",
        "AIL005",
        "read without an explicit protection rule",
    )
}

fn check_failure_handling(core: &AilCore) -> Vec<AilDiagnostic> {
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Failure")
        .filter(|failure| {
            failure
                .attributes
                .get("declared")
                .is_some_and(|value| value == "true")
        })
        .filter(|failure| !has_outgoing_edge(&core.graph, "handles_failure", &failure.id))
        .map(|failure| {
            AilDiagnostic::error(
                "AIL008",
                format!("failure {} is missing declared handling", failure.name),
            )
            .with_source_provenance(node_provenance(core, &failure.id))
            .with_affected_graph_item(format!("node:{}", failure.id))
            .with_repair_suggestion(format!(
                "Add at least one handling bullet to Failure {}.",
                failure.name
            ))
        })
        .collect()
}

fn check_failure_trace_coverage(core: &AilCore) -> Vec<AilDiagnostic> {
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Failure")
        .filter(|failure| {
            failure
                .attributes
                .get("declared")
                .is_some_and(|value| value == "true")
        })
        .filter(|failure| !has_outgoing_edge(&core.graph, "records_trace", &failure.id))
        .map(|failure| {
            AilDiagnostic::error(
                "AIL009",
                format!("failure {} is missing trace coverage", failure.name),
            )
            .with_source_provenance(node_provenance(core, &failure.id))
            .with_affected_graph_item(format!("node:{}", failure.id))
            .with_repair_suggestion(format!(
                "Add a 'the trace records ...' bullet to Failure {}.",
                failure.name
            ))
        })
        .collect()
}

fn check_semantic_node_provenance(core: &AilCore) -> Vec<AilDiagnostic> {
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind != "Provenance")
        .filter(|node| !has_outgoing_edge(&core.graph, "has_provenance", &node.id))
        .map(|node| {
            let kind = node.kind.to_ascii_lowercase();
            AilDiagnostic::error(
                "AIL011",
                format!("{} '{}' is missing provenance", kind, node.name),
            )
            .with_affected_graph_item(format!("node:{}", node.id))
            .with_repair_suggestion(format!("Attach provenance to {kind} '{}'.", node.name))
        })
        .collect()
}

fn check_guarantee_attachment(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Guarantee")
        .filter(|guarantee| {
            !core.graph.edges.iter().any(|edge| {
                edge.kind == "guarantees"
                    && edge.target == guarantee.id
                    && node_by_id.get(&edge.source).is_some_and(|source| {
                        matches!(source.kind.as_str(), "Action" | "Tool" | "SystemComponent")
                    })
            })
        })
        .map(|guarantee| {
            AilDiagnostic::error(
                "AIL012",
                format!(
                    "guarantee '{}' is not attached to an action or tool",
                    guarantee.name
                ),
            )
            .with_source_provenance(node_provenance(core, &guarantee.id))
            .with_affected_graph_item(format!("node:{}", guarantee.id))
            .with_repair_suggestion(format!(
                "Attach guarantee '{}' to an action or tool.",
                guarantee.name
            ))
        })
        .collect()
}

fn check_trace_attachment(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Trace")
        .filter(|trace| {
            !core.graph.edges.iter().any(|edge| {
                edge.kind == "records_trace"
                    && edge.target == trace.id
                    && node_by_id.get(&edge.source).is_some_and(|source| {
                        matches!(
                            source.kind.as_str(),
                            "Action" | "Failure" | "Tool" | "SystemComponent"
                        )
                    })
            })
        })
        .map(|trace| {
            AilDiagnostic::error(
                "AIL013",
                format!(
                    "trace '{}' is not recorded by an action or failure",
                    trace.name
                ),
            )
            .with_source_provenance(node_provenance(core, &trace.id))
            .with_affected_graph_item(format!("node:{}", trace.id))
            .with_repair_suggestion(format!(
                "Record trace '{}' from an action or failure.",
                trace.name
            ))
        })
        .collect()
}

fn check_rule_attachment(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Rule")
        .filter(|rule| {
            !core.graph.edges.iter().any(|edge| {
                edge.kind == "requires"
                    && edge.target == rule.id
                    && node_by_id
                        .get(&edge.source)
                        .is_some_and(|source| matches!(source.kind.as_str(), "Action" | "Tool"))
            })
        })
        .map(|rule| {
            AilDiagnostic::error(
                "AIL014",
                format!("rule '{}' is not required by an action", rule.name),
            )
            .with_source_provenance(node_provenance(core, &rule.id))
            .with_affected_graph_item(format!("node:{}", rule.id))
            .with_repair_suggestion(format!(
                "Attach rule '{}' to an action requirement.",
                rule.name
            ))
        })
        .collect()
}

fn check_effect_attachment(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Effect")
        .filter(|effect| {
            !core.graph.edges.iter().any(|edge| {
                edge.target == effect.id
                    && match edge.kind.as_str() {
                        "reads" | "writes" | "protects_secret" => {
                            node_by_id.get(&edge.source).is_some_and(|source| {
                                matches!(source.kind.as_str(), "Action" | "Tool")
                            })
                        }
                        "performs" => node_by_id
                            .get(&edge.source)
                            .is_some_and(|source| source.kind == "SystemComponent"),
                        "calls" => node_by_id
                            .get(&edge.source)
                            .is_some_and(|source| source.kind == "Tool"),
                        "handles_failure" => node_by_id
                            .get(&edge.source)
                            .is_some_and(|source| source.kind == "Failure"),
                        _ => false,
                    }
            })
        })
        .map(|effect| {
            AilDiagnostic::error(
                "AIL015",
                format!(
                    "effect '{}' is not attached to an action or failure",
                    effect.name
                ),
            )
            .with_source_provenance(node_provenance(core, &effect.id))
            .with_affected_graph_item(format!("node:{}", effect.id))
            .with_repair_suggestion(format!(
                "Attach effect '{}' to an action or failure.",
                effect.name
            ))
        })
        .collect()
}

fn check_secret_attachment(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Secret")
        .filter(|secret| {
            let protects_declared_field = core.graph.edges.iter().any(|edge| {
                edge.kind == "protects_secret"
                    && edge.source == secret.id
                    && node_by_id.get(&edge.target).is_some_and(|target| {
                        matches!(target.kind.as_str(), "Field" | "Input" | "Output")
                    })
            });
            let protected_by_action = core.graph.edges.iter().any(|edge| {
                edge.kind == "protects_secret"
                    && edge.target == secret.id
                    && node_by_id
                        .get(&edge.source)
                        .is_some_and(|source| matches!(source.kind.as_str(), "Action" | "Tool"))
            });
            !(protects_declared_field || protected_by_action)
        })
        .map(|secret| {
            AilDiagnostic::error(
                "AIL016",
                format!(
                    "secret '{}' is not attached to a field or action",
                    secret.name
                ),
            )
            .with_source_provenance(node_provenance(core, &secret.id))
            .with_affected_graph_item(format!("node:{}", secret.id))
            .with_repair_suggestion(format!(
                "Attach secret '{}' to a field or action protection edge.",
                secret.name
            ))
        })
        .collect()
}

fn check_tool_trace_coverage(core: &AilCore) -> Vec<AilDiagnostic> {
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Tool")
        .filter(|tool| !has_outgoing_edge(&core.graph, "records_trace", &tool.id))
        .map(|tool| {
            AilDiagnostic::error(
                "AIL017",
                format!("tool {} is missing audit trace coverage", tool.name),
            )
            .with_source_provenance(node_provenance(core, &tool.id))
            .with_affected_graph_item(format!("node:{}", tool.id))
            .with_repair_suggestion(format!(
                "Add a 'The tool records:' section to tool {}.",
                tool.name
            ))
        })
        .collect()
}

fn check_tool_approval_mentions(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Tool")
        .filter(|tool| !has_outgoing_edge(&core.graph, "requires_approval", &tool.id))
        .filter(|tool| {
            core.graph.edges.iter().any(|edge| {
                edge.source == tool.id
                    && !matches!(edge.kind.as_str(), "has_input" | "has_output")
                    && node_by_id
                        .get(&edge.target)
                        .is_some_and(|target| mentions_approval(&target.name))
            })
        })
        .map(|tool| {
            AilDiagnostic::error(
                "AIL018",
                format!(
                    "tool {} mentions approval but has no explicit approval rule",
                    tool.name
                ),
            )
            .with_source_provenance(node_provenance(core, &tool.id))
            .with_affected_graph_item(format!("node:{}", tool.id))
            .with_repair_suggestion(format!(
                "Add a 'The tool requires approval:' section to tool {}.",
                tool.name
            ))
        })
        .collect()
}

fn mentions_approval(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    normalized.contains("approval") || normalized.contains("approve")
}

fn check_tool_permission_mentions(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Tool")
        .filter(|tool| {
            !core.graph.edges.iter().any(|edge| {
                edge.kind == "requires"
                    && edge.source == tool.id
                    && node_by_id
                        .get(&edge.target)
                        .is_some_and(|target| target.kind == "Permission")
            })
        })
        .filter(|tool| {
            core.graph.edges.iter().any(|edge| {
                edge.source == tool.id
                    && !matches!(edge.kind.as_str(), "has_input" | "has_output")
                    && node_by_id
                        .get(&edge.target)
                        .is_some_and(|target| mentions_permission(&target.name))
            })
        })
        .map(|tool| {
            AilDiagnostic::error(
                "AIL019",
                format!(
                    "tool {} mentions permission but has no explicit permission rule",
                    tool.name
                ),
            )
            .with_source_provenance(node_provenance(core, &tool.id))
            .with_affected_graph_item(format!("node:{}", tool.id))
            .with_repair_suggestion(format!(
                "Add a 'The tool requires permission:' section to tool {}.",
                tool.name
            ))
        })
        .collect()
}

fn mentions_permission(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    normalized.contains("permission") || normalized.contains("may ")
}

fn check_system_effect_capabilities(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "performs")
    {
        let Some(component) = node_by_id.get(&edge.source) else {
            continue;
        };
        if component.kind != "SystemComponent"
            || system_component_has_capability(core, &component.id)
        {
            continue;
        }
        let Some(effect) = node_by_id.get(&edge.target) else {
            continue;
        };
        diagnostics.push(
            AilDiagnostic::error(
                "AIL021",
                format!(
                    "system component {} performs effect '{}' without a declared capability",
                    component.name, effect.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &effect.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add a 'The component requires capability:' section to system component {}.",
                component.name
            )),
        );
    }
    diagnostics
}

fn check_system_effect_resources(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "performs")
    {
        let Some(component) = node_by_id.get(&edge.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(effect) = node_by_id.get(&edge.target) else {
            continue;
        };
        let Some(resource_name) = system_effect_resource_reference(&effect.name) else {
            continue;
        };
        if has_outgoing_edge(&core.graph, "targets_resource", &effect.id) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL022",
                format!(
                    "system component {} effect '{}' targets unknown resource '{}'",
                    component.name, effect.name, resource_name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &effect.id)),
            )
            .with_affected_graph_item(format!("node:{}", effect.id))
            .with_repair_suggestion(format!(
                "Declare resource '{}' on system component {} or update the effect to target a declared resource.",
                resource_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_device_effect_capabilities(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for performs in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "performs")
    {
        let Some(component) = node_by_id.get(&performs.source) else {
            continue;
        };
        if component.kind != "SystemComponent"
            || !system_component_has_capability(core, &component.id)
        {
            continue;
        }
        let Some(effect) = node_by_id.get(&performs.target) else {
            continue;
        };
        for target_edge in core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "targets_resource" && edge.source == effect.id)
        {
            let Some(resource) = node_by_id.get(&target_edge.target) else {
                continue;
            };
            if resource.type_name.as_deref() != Some("Device")
                || system_component_has_capability_for_resource(core, &component.id, &resource.id)
            {
                continue;
            }
            let resource_name = system_resource_display_name(component, resource);
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL023",
                    format!(
                        "system component {} effect '{}' targets device resource '{}' without a matching capability",
                        component.name, effect.name, resource_name
                    ),
                )
                .with_source_provenance(
                    performs
                        .attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &effect.id)),
                )
                .with_affected_graph_item(format!("edge:{}", target_edge.id))
                .with_repair_suggestion(format!(
                    "Add a capability such as 'access {}' to system component {}.",
                    resource_name, component.name
                )),
            );
        }
    }
    diagnostics
}

fn check_system_layout_resources(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for uses_layout in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_layout")
    {
        let Some(component) = node_by_id.get(&uses_layout.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(layout) = node_by_id.get(&uses_layout.target) else {
            continue;
        };
        if has_outgoing_edge(&core.graph, "layouts_resource", &layout.id) {
            continue;
        }
        let resource_name = layout
            .attributes
            .get("resource")
            .cloned()
            .unwrap_or_else(|| layout.name.clone());
        diagnostics.push(
            AilDiagnostic::error(
                "AIL031",
                format!(
                    "system component {} declares layout for unknown resource '{}'",
                    component.name, resource_name
                ),
            )
            .with_source_provenance(node_provenance(core, &layout.id))
            .with_affected_graph_item(format!("node:{}", layout.id))
            .with_repair_suggestion(format!(
                "Declare resource '{}' in 'The component uses:' or update the layout bullet for system component {}.",
                resource_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_allocation_resources(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for uses_allocation in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_allocation")
    {
        let Some(component) = node_by_id.get(&uses_allocation.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(allocation) = node_by_id.get(&uses_allocation.target) else {
            continue;
        };
        if has_outgoing_edge(&core.graph, "allocates_resource", &allocation.id) {
            continue;
        }
        let resource_name = allocation
            .attributes
            .get("resource")
            .cloned()
            .unwrap_or_else(|| allocation.name.clone());
        diagnostics.push(
            AilDiagnostic::error(
                "AIL032",
                format!(
                    "system component {} declares allocation for unknown resource '{}'",
                    component.name, resource_name
                ),
            )
            .with_source_provenance(node_provenance(core, &allocation.id))
            .with_affected_graph_item(format!("node:{}", allocation.id))
            .with_repair_suggestion(format!(
                "Declare resource '{}' in 'The component uses:' or update the allocation bullet for system component {}.",
                resource_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_lock_guards(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for uses_guard in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_lock_guard")
    {
        let Some(component) = node_by_id.get(&uses_guard.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(guard) = node_by_id.get(&uses_guard.target) else {
            continue;
        };
        let resource_name = guard
            .attributes
            .get("resource")
            .cloned()
            .unwrap_or_else(|| system_resource_display_name(component, guard));
        if !has_outgoing_edge(&core.graph, "guards_resource", &guard.id) {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL038",
                    format!(
                        "system component {} declares lock guard for unknown resource '{}'",
                        component.name, resource_name
                    ),
                )
                .with_source_provenance(node_provenance(core, &guard.id))
                .with_affected_graph_item(format!("node:{}", guard.id))
                .with_repair_suggestion(format!(
                    "Declare resource '{}' in 'The component uses:' or update the lock guard bullet for system component {}.",
                    resource_name, component.name
                )),
            );
        }
        if has_outgoing_edge(&core.graph, "uses_lock_resource", &guard.id) {
            continue;
        }
        let lock_name = guard.attributes.get("lock").cloned().unwrap_or_else(|| {
            guard
                .type_name
                .clone()
                .unwrap_or_else(|| system_resource_display_name(component, guard))
        });
        diagnostics.push(
            AilDiagnostic::error(
                "AIL039",
                format!(
                    "system component {} guards resource '{}' with unknown lock resource '{}'",
                    component.name, resource_name, lock_name
                ),
            )
            .with_source_provenance(node_provenance(core, &guard.id))
            .with_affected_graph_item(format!("node:{}", guard.id))
            .with_repair_suggestion(format!(
                "Declare lock resource '{}' in 'The component uses:' or update the lock guard bullet for system component {}.",
                lock_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_interrupt_priority_contexts(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for uses_priority in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_interrupt_priority")
    {
        let Some(component) = node_by_id.get(&uses_priority.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(priority) = node_by_id.get(&uses_priority.target) else {
            continue;
        };
        if has_outgoing_edge(&core.graph, "prioritizes_context", &priority.id) {
            continue;
        }
        let context_name = priority
            .attributes
            .get("context")
            .cloned()
            .unwrap_or_else(|| system_execution_context_name(component, priority));
        diagnostics.push(
            AilDiagnostic::error(
                "AIL034",
                format!(
                    "system component {} configures priority for unknown context '{}'",
                    component.name, context_name
                ),
            )
            .with_source_provenance(node_provenance(core, &priority.id))
            .with_affected_graph_item(format!("node:{}", priority.id))
            .with_repair_suggestion(format!(
                "Add '{}' to 'The component runs in context:' or update the priority bullet for system component {}.",
                context_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_interrupt_mask_contexts(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for uses_mask in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_interrupt_mask")
    {
        let Some(component) = node_by_id.get(&uses_mask.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(mask) = node_by_id.get(&uses_mask.target) else {
            continue;
        };
        if has_outgoing_edge(&core.graph, "masks_context", &mask.id) {
            continue;
        }
        let context_name = mask
            .attributes
            .get("context")
            .cloned()
            .unwrap_or_else(|| system_execution_context_name(component, mask));
        diagnostics.push(
            AilDiagnostic::error(
                "AIL040",
                format!(
                    "system component {} configures interrupt mask for unknown context '{}'",
                    component.name, context_name
                ),
            )
            .with_source_provenance(node_provenance(core, &mask.id))
            .with_affected_graph_item(format!("node:{}", mask.id))
            .with_repair_suggestion(format!(
                "Add '{}' to 'The component runs in context:' or update the interrupt mask bullet for system component {}.",
                context_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_scheduler_task_contexts(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for schedules_task in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "schedules_task")
    {
        let Some(component) = node_by_id.get(&schedules_task.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(task) = node_by_id.get(&schedules_task.target) else {
            continue;
        };
        if has_outgoing_edge(&core.graph, "task_runs_in_context", &task.id) {
            continue;
        }
        let context_name = task
            .attributes
            .get("context")
            .cloned()
            .unwrap_or_else(|| system_execution_context_name(component, task));
        let task_name = system_scheduler_task_name(component, task);
        diagnostics.push(
            AilDiagnostic::error(
                "AIL035",
                format!(
                    "system component {} schedules task '{}' for unknown context '{}'",
                    component.name, task_name, context_name
                ),
            )
            .with_source_provenance(node_provenance(core, &task.id))
            .with_affected_graph_item(format!("node:{}", task.id))
            .with_repair_suggestion(format!(
                "Add '{}' to 'The component runs in context:' or update the task bullet for system component {}.",
                context_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_scheduler_task_priorities(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for uses_priority in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_task_priority")
    {
        let Some(component) = node_by_id.get(&uses_priority.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(priority) = node_by_id.get(&uses_priority.target) else {
            continue;
        };
        if has_outgoing_edge(&core.graph, "prioritizes_task", &priority.id) {
            continue;
        }
        let task_name = priority
            .attributes
            .get("task")
            .cloned()
            .unwrap_or_else(|| system_scheduler_task_name(component, priority));
        diagnostics.push(
            AilDiagnostic::error(
                "AIL036",
                format!(
                    "system component {} configures priority for unknown task '{}'",
                    component.name, task_name
                ),
            )
            .with_source_provenance(node_provenance(core, &priority.id))
            .with_affected_graph_item(format!("node:{}", priority.id))
            .with_repair_suggestion(format!(
                "Add '{}' to 'The component schedules task:' or update the task priority bullet for system component {}.",
                task_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_scheduler_task_timings(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for uses_timing in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "uses_task_timing")
    {
        let Some(component) = node_by_id.get(&uses_timing.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(timing) = node_by_id.get(&uses_timing.target) else {
            continue;
        };
        if has_outgoing_edge(&core.graph, "times_task", &timing.id) {
            continue;
        }
        let task_name = timing
            .attributes
            .get("task")
            .cloned()
            .unwrap_or_else(|| system_scheduler_task_name(component, timing));
        diagnostics.push(
            AilDiagnostic::error(
                "AIL037",
                format!(
                    "system component {} configures timing for unknown task '{}'",
                    component.name, task_name
                ),
            )
            .with_source_provenance(node_provenance(core, &timing.id))
            .with_affected_graph_item(format!("node:{}", timing.id))
            .with_repair_suggestion(format!(
                "Add '{}' to 'The component schedules task:' or update the task timing bullet for system component {}.",
                task_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_interrupt_context_effects(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    let mut interrupt_components = BTreeSet::new();
    for runs_in_context in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "runs_in_context")
    {
        let Some(component) = node_by_id.get(&runs_in_context.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(context) = node_by_id.get(&runs_in_context.target) else {
            continue;
        };
        if system_execution_context_name(component, context) == "interrupt" {
            interrupt_components.insert(component.id.clone());
        }
    }
    for performs in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "performs")
    {
        if !interrupt_components.contains(&performs.source) {
            continue;
        }
        let Some(component) = node_by_id.get(&performs.source) else {
            continue;
        };
        let Some(effect) = node_by_id.get(&performs.target) else {
            continue;
        };
        if !system_effect_blocks_in_interrupt_context(&effect.name) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL033",
                format!(
                    "system component {} performs blocking effect '{}' in interrupt context",
                    component.name, effect.name
                ),
            )
            .with_source_provenance(
                performs
                    .attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &effect.id)),
            )
            .with_affected_graph_item(format!("edge:{}", performs.id))
            .with_repair_suggestion(format!(
                "Move blocking effect '{}' out of interrupt context or remove the 'interrupt' context declaration for system component {}.",
                effect.name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_mutable_effect_ownership(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for performs in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "performs")
    {
        let Some(component) = node_by_id.get(&performs.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(effect) = node_by_id.get(&performs.target) else {
            continue;
        };
        if !system_effect_requires_ownership(&effect.name) {
            continue;
        }
        for target_edge in core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "targets_resource" && edge.source == effect.id)
        {
            let Some(resource) = node_by_id.get(&target_edge.target) else {
                continue;
            };
            if system_component_owns_resource(core, &component.id, &resource.id) {
                continue;
            }
            let resource_name = system_resource_display_name(component, resource);
            if system_effect_moves_resource(&effect.name) {
                diagnostics.push(
                    AilDiagnostic::error(
                        "AIL024",
                        format!(
                            "system component {} moves resource '{}' without ownership",
                            component.name, resource_name
                        ),
                    )
                    .with_source_provenance(
                        performs
                            .attributes
                            .get("provenance")
                            .cloned()
                            .or_else(|| node_provenance(core, &effect.id)),
                    )
                    .with_affected_graph_item(format!("edge:{}", target_edge.id))
                    .with_repair_suggestion(format!(
                        "Add '{}' to 'The component owns:' for system component {} before moving it.",
                        resource_name, component.name
                    )),
                );
                continue;
            }
            if system_component_mutably_borrows_resource(core, &component.id, &resource.id) {
                continue;
            }
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL024",
                    format!(
                        "system component {} mutates resource '{}' without ownership",
                        component.name, resource_name
                    ),
                )
                .with_source_provenance(
                    performs
                        .attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &effect.id)),
                )
                .with_affected_graph_item(format!("edge:{}", target_edge.id))
                .with_repair_suggestion(format!(
                    "Add '{}' to 'The component owns:' for system component {}.",
                    resource_name, component.name
                )),
            );
        }
    }
    diagnostics
}

fn check_system_shared_mutable_borrow_conflicts(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for mutable_edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "mutably_borrows_resource")
    {
        let Some(component) = node_by_id.get(&mutable_edge.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(resource) = node_by_id.get(&mutable_edge.target) else {
            continue;
        };
        if !system_component_borrows_resource(core, &component.id, &resource.id) {
            continue;
        }
        let resource_name = system_resource_display_name(component, resource);
        diagnostics.push(
            AilDiagnostic::error(
                "AIL029",
                format!(
                    "system component {} declares resource '{}' as both shared and mutable borrow",
                    component.name, resource_name
                ),
            )
            .with_source_provenance(node_provenance(core, &component.id))
            .with_affected_graph_item(format!("edge:{}", mutable_edge.id))
            .with_repair_suggestion(format!(
                "Remove '{}' from either 'The component borrows:' or 'The component mutably borrows:' for system component {}.",
                resource_name, component.name
            )),
        );
    }
    diagnostics
}

fn check_system_mutable_borrow_conflicts(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for performs in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "performs")
    {
        let Some(component) = node_by_id.get(&performs.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(effect) = node_by_id.get(&performs.target) else {
            continue;
        };
        if !system_effect_requires_ownership(&effect.name) {
            continue;
        }
        for target_edge in core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "targets_resource" && edge.source == effect.id)
        {
            let Some(resource) = node_by_id.get(&target_edge.target) else {
                continue;
            };
            if !system_component_borrows_resource(core, &component.id, &resource.id) {
                continue;
            }
            let resource_name = system_resource_display_name(component, resource);
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL027",
                    format!(
                        "system component {} mutates borrowed resource '{}'",
                        component.name, resource_name
                    ),
                )
                .with_source_provenance(
                    performs
                        .attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &effect.id)),
                )
                .with_affected_graph_item(format!("edge:{}", target_edge.id))
                .with_repair_suggestion(format!(
                    "Remove '{}' from 'The component borrows:' or stop mutating it in system component {}.",
                    resource_name, component.name
                )),
            );
        }
    }
    diagnostics
}

fn check_system_read_effect_borrowing(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for performs in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "performs")
    {
        let Some(component) = node_by_id.get(&performs.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(effect) = node_by_id.get(&performs.target) else {
            continue;
        };
        if !system_effect_requires_borrowing(&effect.name) {
            continue;
        }
        for target_edge in core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "targets_resource" && edge.source == effect.id)
        {
            let Some(resource) = node_by_id.get(&target_edge.target) else {
                continue;
            };
            if resource.type_name.as_deref() == Some("Device")
                || system_component_owns_resource(core, &component.id, &resource.id)
                || system_component_borrows_resource(core, &component.id, &resource.id)
                || system_component_mutably_borrows_resource(core, &component.id, &resource.id)
            {
                continue;
            }
            let resource_name = system_resource_display_name(component, resource);
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL025",
                    format!(
                        "system component {} reads resource '{}' without ownership or borrowing",
                        component.name, resource_name
                    ),
                )
                .with_source_provenance(
                    performs
                        .attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &effect.id)),
                )
                .with_affected_graph_item(format!("edge:{}", target_edge.id))
                .with_repair_suggestion(format!(
                    "Add '{}' to 'The component borrows:' or 'The component owns:' for system component {}.",
                    resource_name, component.name
                )),
            );
        }
    }
    diagnostics
}

fn check_system_effect_resource_regions(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for performs in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "performs")
    {
        let Some(component) = node_by_id.get(&performs.source) else {
            continue;
        };
        if component.kind != "SystemComponent" {
            continue;
        }
        let Some(effect) = node_by_id.get(&performs.target) else {
            continue;
        };
        for target_edge in core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "targets_resource" && edge.source == effect.id)
        {
            let Some(resource) = node_by_id.get(&target_edge.target) else {
                continue;
            };
            if resource.type_name.as_deref() == Some("Device")
                || has_outgoing_edge(&core.graph, "in_region", &resource.id)
            {
                continue;
            }
            let resource_name = system_resource_display_name(component, resource);
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL026",
                    format!(
                        "system component {} uses resource '{}' without a region",
                        component.name, resource_name
                    ),
                )
                .with_source_provenance(
                    performs
                        .attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &effect.id)),
                )
                .with_affected_graph_item(format!("edge:{}", target_edge.id))
                .with_repair_suggestion(format!(
                    "Add '{} in <region>' to 'The component places:' for system component {}.",
                    resource_name, component.name
                )),
            );
        }
    }
    diagnostics
}

fn check_system_use_after_release(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for component in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "SystemComponent")
    {
        let mut released_resources = BTreeMap::<String, String>::new();
        for performs in core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "performs" && edge.source == component.id)
        {
            let Some(effect) = node_by_id.get(&performs.target) else {
                continue;
            };
            for target_edge in core
                .graph
                .edges
                .iter()
                .filter(|edge| edge.kind == "targets_resource" && edge.source == effect.id)
            {
                let Some(resource) = node_by_id.get(&target_edge.target) else {
                    continue;
                };
                if let Some(release_effect) = released_resources.get(&resource.id) {
                    let resource_name = system_resource_display_name(component, resource);
                    diagnostics.push(
                        AilDiagnostic::error(
                            "AIL028",
                            format!(
                                "system component {} uses resource '{}' after release",
                                component.name, resource_name
                            ),
                        )
                        .with_source_provenance(
                            performs
                                .attributes
                                .get("provenance")
                                .cloned()
                                .or_else(|| node_provenance(core, &effect.id)),
                        )
                        .with_affected_graph_item(format!("edge:{}", target_edge.id))
                        .with_repair_suggestion(format!(
                            "Move '{}' before '{}' or remove the post-release use in system component {}.",
                            effect.name, release_effect, component.name
                        )),
                    );
                    continue;
                }
                if system_effect_releases_resource(&effect.name) {
                    released_resources.insert(resource.id.clone(), effect.name.clone());
                }
            }
        }
    }
    diagnostics
}

fn check_system_use_after_move(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for component in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "SystemComponent")
    {
        let mut moved_resources = BTreeMap::<String, String>::new();
        for performs in core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "performs" && edge.source == component.id)
        {
            let Some(effect) = node_by_id.get(&performs.target) else {
                continue;
            };
            for target_edge in core
                .graph
                .edges
                .iter()
                .filter(|edge| edge.kind == "targets_resource" && edge.source == effect.id)
            {
                let Some(resource) = node_by_id.get(&target_edge.target) else {
                    continue;
                };
                if let Some(move_effect) = moved_resources.get(&resource.id) {
                    let resource_name = system_resource_display_name(component, resource);
                    diagnostics.push(
                        AilDiagnostic::error(
                            "AIL030",
                            format!(
                                "system component {} uses resource '{}' after move",
                                component.name, resource_name
                            ),
                        )
                        .with_source_provenance(
                            performs
                                .attributes
                                .get("provenance")
                                .cloned()
                                .or_else(|| node_provenance(core, &effect.id)),
                        )
                        .with_affected_graph_item(format!("edge:{}", target_edge.id))
                        .with_repair_suggestion(format!(
                            "Move '{}' before '{}' or remove the post-move use in system component {}.",
                            effect.name, move_effect, component.name
                        )),
                    );
                    continue;
                }
                if system_effect_moves_resource(&effect.name) {
                    moved_resources.insert(resource.id.clone(), effect.name.clone());
                }
            }
        }
    }
    diagnostics
}

fn system_component_has_capability(core: &AilCore, component_id: &str) -> bool {
    let node_by_id = graph_node_by_id(core);
    core.graph.edges.iter().any(|edge| {
        edge.kind == "requires"
            && edge.source == component_id
            && node_by_id
                .get(&edge.target)
                .is_some_and(|target| target.kind == "Capability")
    })
}

fn system_component_has_capability_for_resource(
    core: &AilCore,
    component_id: &str,
    resource_id: &str,
) -> bool {
    let node_by_id = graph_node_by_id(core);
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "requires" && edge.source == component_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .filter(|capability| capability.kind == "Capability")
        .any(|capability| {
            core.graph.edges.iter().any(|edge| {
                edge.kind == "authorizes_resource"
                    && edge.source == capability.id
                    && edge.target == resource_id
            })
        })
}

fn system_component_owns_resource(core: &AilCore, component_id: &str, resource_id: &str) -> bool {
    core.graph.edges.iter().any(|edge| {
        edge.kind == "owns_resource" && edge.source == component_id && edge.target == resource_id
    })
}

fn system_component_borrows_resource(
    core: &AilCore,
    component_id: &str,
    resource_id: &str,
) -> bool {
    core.graph.edges.iter().any(|edge| {
        edge.kind == "borrows_resource" && edge.source == component_id && edge.target == resource_id
    })
}

fn system_component_mutably_borrows_resource(
    core: &AilCore,
    component_id: &str,
    resource_id: &str,
) -> bool {
    core.graph.edges.iter().any(|edge| {
        edge.kind == "mutably_borrows_resource"
            && edge.source == component_id
            && edge.target == resource_id
    })
}

fn system_effect_requires_ownership(effect: &str) -> bool {
    let effect = trim_sentence(effect);
    [
        "write ", "release ", "free ", "unmap ", "pin ", "unpin ", "append ", "delete ", "move ",
    ]
    .iter()
    .any(|verb| effect.starts_with(verb))
}

fn system_effect_requires_borrowing(effect: &str) -> bool {
    trim_sentence(effect).starts_with("read ")
}

fn system_effect_releases_resource(effect: &str) -> bool {
    let effect = trim_sentence(effect);
    effect.starts_with("release ") || effect.starts_with("free ")
}

fn system_effect_moves_resource(effect: &str) -> bool {
    trim_sentence(effect).starts_with("move ")
}

fn system_effect_blocks_in_interrupt_context(effect: &str) -> bool {
    let verb = trim_sentence(effect)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(verb.as_str(), "wait" | "sleep" | "block" | "park")
}

fn system_execution_context_name(
    component: &crate::core_model::Node,
    context: &crate::core_model::Node,
) -> String {
    context
        .attributes
        .get("context")
        .cloned()
        .unwrap_or_else(|| {
            context
                .name
                .strip_prefix(&format!("{}.", component.name))
                .unwrap_or(&context.name)
                .to_string()
        })
}

fn system_scheduler_task_name(
    component: &crate::core_model::Node,
    task: &crate::core_model::Node,
) -> String {
    task.name
        .strip_prefix(&format!("{}.", component.name))
        .unwrap_or(&task.name)
        .to_string()
}

fn system_resource_display_name(
    component: &crate::core_model::Node,
    resource: &crate::core_model::Node,
) -> String {
    resource
        .name
        .strip_prefix(&format!("{}.", component.name))
        .unwrap_or(&resource.name)
        .to_string()
}

fn check_tool_secret_output_disclosure(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "has_output")
    {
        let Some(tool) = node_by_id.get(&edge.source) else {
            continue;
        };
        if tool.kind != "Tool" {
            continue;
        }
        let Some(output) = node_by_id.get(&edge.target) else {
            continue;
        };
        let Some(type_name) = output.type_name.as_deref() else {
            continue;
        };
        if !type_contains_secret(type_name)
            || tool_has_reveal_permission(core, &tool.id, output_name_for_permission(output))
        {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL020",
                format!(
                    "output {} discloses secret type '{}' without reveal permission",
                    output.name, type_name
                ),
            )
            .with_source_provenance(node_provenance(core, &output.id))
            .with_affected_graph_item(format!("node:{}", output.id))
            .with_repair_suggestion(format!(
                "Change output {} to a non-secret redacted type or add an explicit reveal permission.",
                output.name
            )),
        );
    }
    diagnostics
}

fn output_name_for_permission(output: &crate::core_model::Node) -> &str {
    output
        .name
        .split_once('.')
        .map(|(_, name)| name)
        .unwrap_or(&output.name)
}

fn tool_has_reveal_permission(core: &AilCore, tool_id: &str, output_name: &str) -> bool {
    let node_by_id = graph_node_by_id(core);
    let output_name = output_name.to_ascii_lowercase();
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "requires" && edge.source == tool_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .filter(|node| node.kind == "Permission")
        .any(|permission| {
            let text = permission.name.to_ascii_lowercase();
            (text.contains("reveal") || text.contains("disclose")) && text.contains(&output_name)
        })
}

fn check_secret_access_protection(
    core: &AilCore,
    edge_kind: &str,
    code: &str,
    description: &str,
) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == edge_kind)
    {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        let Some(target) = node_by_id.get(&edge.target) else {
            continue;
        };
        if target.kind != "Field"
            || target
                .attributes
                .get("secret")
                .is_none_or(|value| value != "true")
        {
            continue;
        }
        let action_protects_target = core.graph.edges.iter().any(|protection| {
            protection.kind == "protects_secret"
                && protection.source == action.id
                && protection.target == target.id
        });
        if !action_protects_target {
            diagnostics.push(
                AilDiagnostic::error(
                    code,
                    format!("secret field {} is {description}", target.name),
                )
                .with_source_provenance(
                    edge.attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &target.id)),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id))
                .with_repair_suggestion(format!(
                    "Add a 'the system does not reveal {}' protection bullet to action {}.",
                    target.name, action.name
                )),
            );
        }
    }
    diagnostics
}

fn check_unknown_field_references(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let thing_names = core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Thing")
        .map(|node| node.name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut diagnostics = Vec::new();

    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| matches!(edge.kind.as_str(), "reads" | "writes"))
    {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        let Some(target) = node_by_id.get(&edge.target) else {
            continue;
        };
        if target.kind != "Effect" || !looks_like_field_reference(&target.name, &thing_names) {
            continue;
        }
        let verb = if edge.kind == "reads" {
            "reads"
        } else {
            "writes"
        };
        let bullet_kind = if edge.kind == "reads" {
            "read"
        } else {
            "write"
        };
        diagnostics.push(
            AilDiagnostic::error(
                "AIL004",
                format!(
                    "action {} {verb} unknown field reference '{}'",
                    action.name, target.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &target.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Declare field '{}' on the referenced thing or update the {bullet_kind} bullet to an existing field.",
                target.name
            )),
        );
    }
    diagnostics
}

fn looks_like_field_reference(text: &str, thing_names: &[String]) -> bool {
    let normalized = text
        .trim()
        .trim_start_matches("the ")
        .trim_start_matches("a ")
        .trim_start_matches("an ")
        .to_ascii_lowercase();
    thing_names.iter().any(|thing| {
        normalized == *thing
            || normalized.starts_with(&format!("{thing} "))
            || normalized.starts_with(&format!("{thing}."))
    })
}

fn graph_node_by_id(core: &AilCore) -> BTreeMap<String, crate::core_model::Node> {
    core.graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.clone()))
        .collect()
}

fn node_provenance(core: &AilCore, node_id: &str) -> Option<String> {
    let node_by_id = graph_node_by_id(core);
    if let Some(provenance) = node_by_id
        .get(node_id)
        .and_then(|node| node.attributes.get("provenance"))
    {
        return Some(provenance.clone());
    }
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "has_provenance" && edge.source == node_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .find(|node| node.kind == "Provenance")
        .map(|node| node.name.clone())
}

fn existence_requirement_reference(rule: &str) -> Option<String> {
    let lower = rule.to_ascii_lowercase();
    let end = lower.find(" to exist").or_else(|| {
        lower
            .contains(" and ")
            .then(|| lower.find(" exists"))
            .flatten()
    })?;
    let reference = rule[..end]
        .trim()
        .trim_start_matches("the ")
        .trim()
        .to_string();
    (!reference.is_empty()).then_some(reference)
}

fn existence_requirement_runtime_key(document: &AilDocument, reference: &str) -> String {
    referenced_runtime_field_key(document, reference)
        .unwrap_or_else(|| format!("{}.id", runtime_subject_key(reference)))
}

fn requirement_field_reference_text(rule: &str) -> Option<String> {
    let (field_text, _) = rule
        .split_once(" not to be ")
        .or_else(|| rule.split_once(" to be "))
        .or_else(|| rule.split_once(" is "))?;
    let field_text = normalized_field_reference_text(field_text);
    (!field_text.is_empty()).then_some(field_text)
}

fn referenced_core_field_name(field_names: &[String], text: &str) -> Option<String> {
    let normalized = normalized_field_reference_text(text).to_ascii_lowercase();
    let mut qualified_matches = Vec::new();
    let mut field_matches = Vec::new();
    for field_name in field_names {
        let Some((thing_name, field_text)) = field_name.split_once('.') else {
            continue;
        };
        let thing_text = thing_name.to_ascii_lowercase();
        let field_text = field_text.to_ascii_lowercase();
        let qualified = format!("{thing_text} {field_text}");
        if normalized.contains(&qualified) {
            qualified_matches.push((qualified.len(), field_name.clone()));
        } else if normalized.contains(&field_text) {
            field_matches.push(field_name.clone());
        }
    }
    qualified_matches.sort_by_key(|(len, _)| std::cmp::Reverse(*len));
    if let Some((_, field_name)) = qualified_matches.into_iter().next() {
        return Some(field_name);
    }
    field_matches.sort();
    field_matches.dedup();
    (field_matches.len() == 1).then(|| field_matches.remove(0))
}

fn normalized_field_reference_text(text: &str) -> String {
    text.trim()
        .trim_start_matches("the ")
        .trim_start_matches("a ")
        .trim_start_matches("an ")
        .trim()
        .to_string()
}

fn is_known_ail_type(type_name: &str, declared_types: &BTreeSet<&str>) -> bool {
    let type_name = type_name.trim();
    if matches!(
        type_name,
        "Text"
            | "Time"
            | "Duration"
            | "Bool"
            | "Int"
            | "Decimal"
            | "Money"
            | "Buffer"
            | "Device"
            | "AIL-Core graph"
            | "permission inference policy"
            | "Diagnostic"
    ) {
        return true;
    }
    if declared_types.contains(type_name) {
        return true;
    }
    if let Some(values) = generic_inner(type_name, "State") {
        return values
            .split(',')
            .map(str::trim)
            .all(|value| !value.is_empty());
    }
    for wrapper in ["Option", "List", "Secret"] {
        if let Some(inner) = generic_inner(type_name, wrapper) {
            return is_known_ail_type(inner, declared_types);
        }
    }
    false
}

fn parse_application_line(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("The application ")?;
    let (name, purpose) = rest.split_once(" manages ")?;
    Some((
        name.trim().to_string(),
        purpose.trim().trim_end_matches('.').to_string(),
    ))
}

fn parse_thing_header(line: &str) -> Option<String> {
    let line = line.strip_suffix(" has:")?;
    if let Some(name) = line.strip_prefix("A ") {
        Some(name.trim().to_string())
    } else {
        line.strip_prefix("An ").map(|name| name.trim().to_string())
    }
}

fn parse_action_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Action: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_tool_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Tool: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_compiler_pass_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Compiler pass: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_system_component_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("System component: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_compiler_pass_purpose_line(line: &str) -> Option<String> {
    if line.ends_with(':') {
        return None;
    }
    line.strip_prefix("The pass ")
        .map(|rest| format!("The pass {}", trim_sentence(rest)))
}

fn parse_tool_section(line: &str) -> Option<ToolSection> {
    if line.starts_with("The AI Agent may request ") && line.ends_with(" when:") {
        return Some(ToolSection::Requirements);
    }
    match line {
        "The tool needs:" => Some(ToolSection::Inputs),
        "The tool produces:" => Some(ToolSection::Outputs),
        "The tool can:" => Some(ToolSection::Capabilities),
        "The tool must not:" => Some(ToolSection::Protections),
        "The tool requires permission:" => Some(ToolSection::Permissions),
        "The tool requires approval:" => Some(ToolSection::Approvals),
        "The tool records:" => Some(ToolSection::Traces),
        "The tool guarantees:" => Some(ToolSection::Guarantees),
        _ => None,
    }
}

fn parse_compiler_pass_section(line: &str) -> Option<CompilerPassSection> {
    if line.starts_with("When the compiler runs ") && line.ends_with(':') {
        return Some(CompilerPassSection::Body);
    }
    match line {
        "The pass needs:" => Some(CompilerPassSection::Inputs),
        "The pass produces:" => Some(CompilerPassSection::Outputs),
        _ => None,
    }
}

fn parse_system_section(line: &str) -> Option<SystemSection> {
    match line {
        "The component uses:" => Some(SystemSection::Resources),
        "The component owns:" => Some(SystemSection::Ownership),
        "The component borrows:" => Some(SystemSection::Borrowing),
        "The component mutably borrows:" => Some(SystemSection::MutableBorrowing),
        "The component places:" => Some(SystemSection::Regions),
        "The component lays out:" => Some(SystemSection::Layouts),
        "The component allocates:" => Some(SystemSection::Allocations),
        "The component guards:" => Some(SystemSection::LockGuards),
        "The component runs in context:" => Some(SystemSection::ExecutionContexts),
        "The component sets interrupt priority:" => Some(SystemSection::InterruptPriorities),
        "The component masks interrupt:" => Some(SystemSection::InterruptMasks),
        "The component schedules task:" => Some(SystemSection::SchedulerTasks),
        "The component sets task priority:" => Some(SystemSection::SchedulerTaskPriorities),
        "The component sets task timing:" => Some(SystemSection::SchedulerTaskTimings),
        "The component requires capability:" => Some(SystemSection::Capabilities),
        "The component performs:" => Some(SystemSection::Effects),
        "The component records:" => Some(SystemSection::Traces),
        "The component guarantees:" => Some(SystemSection::Guarantees),
        _ => None,
    }
}

fn parse_when_line(line: &str) -> Option<String> {
    let trigger = line.strip_prefix("When ")?;
    Some(trigger.trim().trim_end_matches(':').to_string())
}

fn parse_failure_header(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("Failure ")?;
    let (name, condition) = rest.split_once(" happens when ")?;
    Some((
        name.trim().to_string(),
        condition.trim().trim_end_matches(':').to_string(),
    ))
}

fn parse_field_bullet(
    document: &mut AilDocument,
    thing_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let Some((name, type_name)) = bullet.split_once(':') else {
        return Err(format!("line {line_number}: expected '<field>: <type>'"));
    };
    let name = name.trim().to_string();
    let type_name = normalize_type_name(type_name);
    let is_secret = type_contains_secret(&type_name);
    let field = AilField {
        name: name.clone(),
        type_name,
        is_secret,
        provenance: format!("field:{thing_name}.{name}"),
    };
    let thing = document
        .things
        .get_mut(thing_name)
        .ok_or_else(|| format!("line {line_number}: unknown thing '{thing_name}'"))?;
    thing.fields.insert(name, field);
    Ok(())
}

fn parse_tool_bullet(
    document: &mut AilDocument,
    tool_name: &str,
    section: ToolSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let Some(tool) = document.tools.get_mut(tool_name) else {
        return Ok(());
    };
    match section {
        ToolSection::Requirements => tool.requirements.push(trim_sentence(bullet)),
        ToolSection::Inputs => {
            let slot = parse_tool_slot(tool_name, "input", bullet, line_number)?;
            tool.inputs.insert(slot.name.clone(), slot);
        }
        ToolSection::Outputs => {
            let slot = parse_tool_slot(tool_name, "output", bullet, line_number)?;
            tool.outputs.insert(slot.name.clone(), slot);
        }
        ToolSection::Capabilities => {
            if let Some(text) = bullet.strip_prefix("read ") {
                tool.reads.push(trim_sentence(text));
            } else if let Some(text) = bullet.strip_prefix("call ") {
                tool.calls.push(trim_sentence(text));
            } else if let Some(text) = bullet.strip_prefix("write ") {
                tool.writes.push(trim_sentence(text));
            } else if let Some(text) = bullet.strip_prefix("create ") {
                tool.writes.push(trim_sentence(text));
            } else {
                tool.writes.push(trim_sentence(bullet));
            }
        }
        ToolSection::Protections => {
            if let Some(text) = bullet.strip_prefix("reveal ") {
                tool.secret_protections.push(trim_sentence(text));
            } else {
                tool.guarantees.push(trim_sentence(bullet));
            }
        }
        ToolSection::Permissions => tool.permissions.push(trim_sentence(bullet)),
        ToolSection::Approvals => tool.approvals.push(trim_sentence(bullet)),
        ToolSection::Traces => tool.traces.push(trim_sentence(bullet)),
        ToolSection::Guarantees => tool.guarantees.push(trim_sentence(bullet)),
    }
    Ok(())
}

fn parse_tool_slot(
    tool_name: &str,
    kind: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilToolSlot, String> {
    let Some((name, type_name)) = bullet.split_once(':') else {
        return Err(format!("line {line_number}: expected '<{kind}>: <type>'"));
    };
    let name = name.trim().to_string();
    let type_name = normalize_type_name(type_name);
    let is_secret = type_contains_secret(&type_name);
    Ok(AilToolSlot {
        provenance: format!("tool:{tool_name}.{kind}:{name}"),
        name,
        type_name,
        is_secret,
    })
}

fn parse_compiler_pass_bullet(
    document: &mut AilDocument,
    pass_name: &str,
    section: CompilerPassSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let Some(pass) = document.compiler_passes.get_mut(pass_name) else {
        return Ok(());
    };
    match section {
        CompilerPassSection::Inputs => {
            let value = parse_pass_value(pass_name, "input", bullet, line_number)?;
            pass.inputs.insert(value.name.clone(), value);
        }
        CompilerPassSection::Outputs => {
            let value = parse_pass_value(pass_name, "output", bullet, line_number)?;
            pass.outputs.insert(value.name.clone(), value);
        }
        CompilerPassSection::Body => parse_compiler_pass_body_bullet(pass, bullet),
    }
    Ok(())
}

fn parse_system_bullet(
    document: &mut AilDocument,
    component_name: &str,
    section: SystemSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let Some(component) = document.system_components.get_mut(component_name) else {
        return Ok(());
    };
    match section {
        SystemSection::Resources => {
            let resource = parse_system_resource(component_name, bullet, line_number)?;
            component.resources.insert(resource.name.clone(), resource);
        }
        SystemSection::Ownership => component.owned_resources.push(trim_sentence(bullet)),
        SystemSection::Borrowing => component.borrowed_resources.push(trim_sentence(bullet)),
        SystemSection::MutableBorrowing => component
            .mutably_borrowed_resources
            .push(trim_sentence(bullet)),
        SystemSection::Regions => {
            let placement = parse_system_resource_region(component_name, bullet, line_number)?;
            component.resource_regions.push(placement);
        }
        SystemSection::Layouts => {
            let layout = parse_system_resource_layout(component_name, bullet, line_number)?;
            component.resource_layouts.push(layout);
        }
        SystemSection::Allocations => {
            let allocation = parse_system_resource_allocation(component_name, bullet, line_number)?;
            component.resource_allocations.push(allocation);
        }
        SystemSection::LockGuards => {
            let guard = parse_system_lock_guard(component_name, bullet, line_number)?;
            component.lock_guards.push(guard);
        }
        SystemSection::ExecutionContexts => {
            let context = parse_system_execution_context(component_name, bullet, line_number)?;
            component.execution_contexts.push(context);
        }
        SystemSection::InterruptPriorities => {
            let priority = parse_system_interrupt_priority(component_name, bullet, line_number)?;
            component.interrupt_priorities.push(priority);
        }
        SystemSection::InterruptMasks => {
            let mask = parse_system_interrupt_mask(component_name, bullet, line_number)?;
            component.interrupt_masks.push(mask);
        }
        SystemSection::SchedulerTasks => {
            let task = parse_system_scheduler_task(component_name, bullet, line_number)?;
            component.scheduler_tasks.push(task);
        }
        SystemSection::SchedulerTaskPriorities => {
            let priority =
                parse_system_scheduler_task_priority(component_name, bullet, line_number)?;
            component.scheduler_task_priorities.push(priority);
        }
        SystemSection::SchedulerTaskTimings => {
            let timing = parse_system_scheduler_task_timing(component_name, bullet, line_number)?;
            component.scheduler_task_timings.push(timing);
        }
        SystemSection::Capabilities => component.capabilities.push(trim_sentence(bullet)),
        SystemSection::Effects => component.effects.push(trim_sentence(bullet)),
        SystemSection::Traces => component.traces.push(trim_sentence(bullet)),
        SystemSection::Guarantees => component.guarantees.push(trim_sentence(bullet)),
    }
    Ok(())
}

fn parse_system_resource(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemResource, String> {
    let Some((name, type_name)) = bullet.split_once(':') else {
        return Err(format!("line {line_number}: expected '<resource>: <type>'"));
    };
    let name = name.trim().to_string();
    Ok(AilSystemResource {
        provenance: format!("system_component:{component_name}.resource:{name}"),
        name,
        type_name: normalize_type_name(type_name),
    })
}

fn parse_system_resource_region(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemResourceRegion, String> {
    let Some((resource_name, region_name)) = bullet.split_once(" in ") else {
        return Err(format!(
            "line {line_number}: expected '<resource> in <region>'"
        ));
    };
    let resource_name = trim_sentence(resource_name);
    let region_name = trim_sentence(region_name);
    Ok(AilSystemResourceRegion {
        provenance: format!("system_component:{component_name}.region:{region_name}"),
        resource_name,
        region_name,
    })
}

fn parse_system_resource_layout(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemResourceLayout, String> {
    let Some((resource_name, layout)) = bullet.split_once(':') else {
        return Err(format!(
            "line {line_number}: expected '<resource>: <layout rule>'"
        ));
    };
    let resource_name = trim_sentence(resource_name);
    let layout = trim_sentence(layout);
    Ok(AilSystemResourceLayout {
        provenance: format!("system_component:{component_name}.layout:{resource_name}"),
        resource_name,
        layout,
    })
}

fn parse_system_resource_allocation(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemResourceAllocation, String> {
    let Some((resource_name, placement)) = bullet.split_once(':') else {
        return Err(format!(
            "line {line_number}: expected '<resource>: <allocation placement>'"
        ));
    };
    let resource_name = trim_sentence(resource_name);
    let placement = trim_sentence(placement);
    Ok(AilSystemResourceAllocation {
        provenance: format!("system_component:{component_name}.allocation:{resource_name}"),
        resource_name,
        placement,
    })
}

fn parse_system_lock_guard(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemLockGuard, String> {
    let Some((resource_name, lock_name)) = bullet.split_once(" with ") else {
        return Err(format!(
            "line {line_number}: expected '<resource> with <lock resource>'"
        ));
    };
    let resource_name = trim_sentence(resource_name);
    let lock_name = trim_sentence(lock_name);
    Ok(AilSystemLockGuard {
        provenance: format!("system_component:{component_name}.lock_guard:{resource_name}"),
        resource_name,
        lock_name,
    })
}

fn parse_system_execution_context(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemExecutionContext, String> {
    let name = trim_sentence(bullet);
    if name.is_empty() {
        return Err(format!("line {line_number}: expected '<context>'"));
    }
    Ok(AilSystemExecutionContext {
        provenance: format!("system_component:{component_name}.context:{name}"),
        name,
    })
}

fn parse_system_interrupt_priority(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemInterruptPriority, String> {
    let Some((context_name, priority)) = bullet.split_once(':') else {
        return Err(format!(
            "line {line_number}: expected '<context>: <priority>'"
        ));
    };
    let context_name = trim_sentence(context_name);
    let priority = trim_sentence(priority);
    Ok(AilSystemInterruptPriority {
        provenance: format!("system_component:{component_name}.priority:{context_name}"),
        context_name,
        priority,
    })
}

fn parse_system_interrupt_mask(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemInterruptMask, String> {
    let Some((context_name, mask)) = bullet.split_once(':') else {
        return Err(format!(
            "line {line_number}: expected '<context>: <mask rule>'"
        ));
    };
    let context_name = trim_sentence(context_name);
    let mask = trim_sentence(mask);
    Ok(AilSystemInterruptMask {
        provenance: format!("system_component:{component_name}.interrupt_mask:{context_name}"),
        context_name,
        mask,
    })
}

fn parse_system_scheduler_task(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemSchedulerTask, String> {
    let Some((task_name, context_name)) = bullet.split_once(':') else {
        return Err(format!("line {line_number}: expected '<task>: <context>'"));
    };
    let task_name = trim_sentence(task_name);
    let context_name = trim_sentence(context_name);
    Ok(AilSystemSchedulerTask {
        provenance: format!("system_component:{component_name}.task:{task_name}"),
        task_name,
        context_name,
    })
}

fn parse_system_scheduler_task_priority(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemSchedulerTaskPriority, String> {
    let Some((task_name, priority)) = bullet.split_once(':') else {
        return Err(format!("line {line_number}: expected '<task>: <priority>'"));
    };
    let task_name = trim_sentence(task_name);
    let priority = trim_sentence(priority);
    Ok(AilSystemSchedulerTaskPriority {
        provenance: format!("system_component:{component_name}.task_priority:{task_name}"),
        task_name,
        priority,
    })
}

fn parse_system_scheduler_task_timing(
    component_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilSystemSchedulerTaskTiming, String> {
    let Some((task_name, timing)) = bullet.split_once(':') else {
        return Err(format!(
            "line {line_number}: expected '<task>: deadline <duration>, budget <duration>'"
        ));
    };
    let timing = trim_sentence(timing);
    let Some(after_deadline) = timing.strip_prefix("deadline ") else {
        return Err(format!(
            "line {line_number}: expected '<task>: deadline <duration>, budget <duration>'"
        ));
    };
    let Some((deadline, budget)) = after_deadline.split_once(", budget ") else {
        return Err(format!(
            "line {line_number}: expected '<task>: deadline <duration>, budget <duration>'"
        ));
    };
    let task_name = trim_sentence(task_name);
    let deadline = trim_sentence(deadline);
    let budget = trim_sentence(budget);
    Ok(AilSystemSchedulerTaskTiming {
        provenance: format!("system_component:{component_name}.task_timing:{task_name}"),
        task_name,
        deadline,
        budget,
    })
}

fn parse_pass_value(
    pass_name: &str,
    kind: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilPassValue, String> {
    let Some((name, type_name)) = bullet.split_once(':') else {
        return Err(format!("line {line_number}: expected '<{kind}>: <type>'"));
    };
    let name = name.trim().to_string();
    Ok(AilPassValue {
        provenance: format!("compiler_pass:{pass_name}.{kind}:{name}"),
        name,
        type_name: normalize_type_name(type_name),
    })
}

fn parse_compiler_pass_body_bullet(pass: &mut AilCompilerPass, bullet: &str) {
    if let Some(text) = bullet.strip_prefix("the system reads ") {
        pass.reads.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system writes ") {
        pass.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system adds ") {
        pass.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system emits ") {
        pass.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system guarantees ") {
        pass.guarantees.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system records a trace event named ") {
        pass.traces.push(trim_sentence(text));
    } else if let Some((_, text)) = bullet.split_once(", the system adds ") {
        pass.writes.push(trim_sentence(text));
    } else if let Some((_, text)) = bullet.split_once(", the system emits ") {
        pass.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system ") {
        pass.steps.push(trim_sentence(text));
    } else {
        pass.steps.push(trim_sentence(bullet));
    }
}

fn parse_action_bullet(document: &mut AilDocument, action_name: &str, bullet: &str) {
    let Some(action) = document.actions.get_mut(action_name) else {
        return;
    };
    if let Some(text) = bullet.strip_prefix("the system requires ") {
        action.requirements.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system reads ") {
        action.reads.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system changes ") {
        action.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system creates ") {
        action.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system calls ") {
        action.writes.push(format!("call {}", trim_sentence(text)));
    } else if let Some(text) = bullet.strip_prefix("the system records a trace event named ") {
        action.traces.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system records ") {
        action.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system guarantees ") {
        action.guarantees.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system does not reveal ") {
        action.secret_protections.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("if ") {
        action.failures.push(trim_sentence(text));
    }
}

fn parse_failure_bullet(document: &mut AilDocument, failure_name: &str, bullet: &str) {
    let Some(failure) = document.failures.get_mut(failure_name) else {
        return;
    };
    if let Some(text) = bullet.strip_prefix("the trace records ") {
        failure.traces.push(trim_sentence(text));
    } else {
        failure.handling.push(trim_sentence(bullet));
    }
}

fn wrapped_bullet(text: &str) -> String {
    text.trim().to_string()
}

fn trim_sentence(text: &str) -> String {
    text.trim().trim_end_matches('.').to_string()
}

fn type_contains_secret(type_name: &str) -> bool {
    let type_name = normalize_type_name(type_name);
    type_name.contains("Secret<") || type_name == "Secret"
}

fn normalize_type_name(type_name: &str) -> String {
    let type_name = type_name.trim();
    if type_name.eq_ignore_ascii_case("String") {
        return "Text".to_string();
    }
    if let Some(inner) = type_name.strip_prefix("Secret ") {
        return format!("Secret<{}>", normalize_type_name(inner));
    }
    if let Some(inner) = type_name.strip_prefix("List ") {
        return format!("List<{}>", normalize_type_name(inner));
    }
    if let Some(inner) = type_name.strip_prefix("Option ") {
        return format!("Option<{}>", normalize_type_name(inner));
    }
    if let Some(values) = type_name.strip_prefix("Enum:") {
        let values = values
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        return format!("State<{values}>");
    }
    for wrapper in ["Secret", "List", "Option"] {
        if let Some(inner) = generic_inner(type_name, wrapper) {
            return format!("{wrapper}<{}>", normalize_type_name(inner));
        }
    }
    type_name.to_string()
}

fn generic_inner<'a>(type_name: &'a str, wrapper: &str) -> Option<&'a str> {
    let prefix = format!("{wrapper}<");
    let inner = type_name.strip_prefix(&prefix)?.strip_suffix('>')?;
    Some(inner.trim())
}

fn to_pascal_case(text: &str) -> String {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn action_name_from_label(label: &str) -> String {
    if let Some((namespace, name)) = label.split_once('.') {
        let namespace = namespace.trim();
        let name = name.trim();
        if !namespace.is_empty() && !name.is_empty() {
            return format!("{namespace}.{}", to_pascal_case(name));
        }
    }
    to_pascal_case(label)
}

fn title_from_pascal_case(text: &str) -> String {
    let mut output = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index > 0 && ch.is_ascii_uppercase() {
            output.push(' ');
        }
        output.push(ch);
    }
    output
}

fn infer_action_name_from_trigger(trigger: &str) -> String {
    if let Some(action) = trigger.strip_prefix("the scheduler ") {
        return to_pascal_case(action);
    }
    to_pascal_case(trigger)
}

fn resolve_field_or_effect(
    graph: &mut Graph,
    document: &AilDocument,
    text: &str,
) -> crate::core_model::Node {
    if let Some(field_node) = find_referenced_field(graph, document, text) {
        return field_node;
    }
    graph.add_node("Effect", text, None, BTreeMap::new())
}

fn resolve_secret_target(
    graph: &mut Graph,
    document: &AilDocument,
    text: &str,
) -> crate::core_model::Node {
    find_referenced_field(graph, document, text)
        .unwrap_or_else(|| graph.add_node("Secret", text, None, BTreeMap::new()))
}

fn resolve_tool_secret_target(
    graph: &mut Graph,
    tool: &AilTool,
    text: &str,
) -> crate::core_model::Node {
    let normalized = text.to_ascii_lowercase();
    tool.inputs
        .values()
        .filter(|input| input.is_secret)
        .find(|input| normalized.contains(&input.name.to_ascii_lowercase()))
        .and_then(|input| graph.find_node("Secret", &format!("{}.{}", tool.name, input.name)))
        .cloned()
        .unwrap_or_else(|| graph.add_node("Secret", text, None, BTreeMap::new()))
}

fn resolve_pass_value_or_effect(
    graph: &mut Graph,
    pass: &AilCompilerPass,
    text: &str,
) -> crate::core_model::Node {
    let normalized = text.to_ascii_lowercase();
    pass.inputs
        .values()
        .chain(pass.outputs.values())
        .find(|value| normalized.contains(&value.name.to_ascii_lowercase()))
        .and_then(|value| graph.find_node("Value", &format!("{}.{}", pass.name, value.name)))
        .cloned()
        .unwrap_or_else(|| graph.add_node("Effect", text, None, BTreeMap::new()))
}

fn resolve_system_effect_resource(
    graph: &Graph,
    component: &AilSystemComponent,
    effect: &str,
) -> Option<crate::core_model::Node> {
    let resource_name = system_effect_resource_reference(effect)?;
    resolve_system_component_resource(graph, component, &resource_name)
}

fn resolve_system_capability_resource(
    graph: &Graph,
    component: &AilSystemComponent,
    capability: &str,
) -> Option<crate::core_model::Node> {
    let resource_name = system_capability_resource_reference(capability)?;
    resolve_system_component_resource(graph, component, &resource_name)
}

fn resolve_system_component_resource(
    graph: &Graph,
    component: &AilSystemComponent,
    resource_name: &str,
) -> Option<crate::core_model::Node> {
    component
        .resources
        .values()
        .filter(|resource| resource.name.eq_ignore_ascii_case(resource_name))
        .find_map(|resource| {
            graph
                .find_node("Resource", &format!("{}.{}", component.name, resource.name))
                .cloned()
        })
}

fn resolve_system_component_execution_context(
    graph: &Graph,
    component: &AilSystemComponent,
    context_name: &str,
) -> Option<crate::core_model::Node> {
    component
        .execution_contexts
        .iter()
        .filter(|context| context.name.eq_ignore_ascii_case(context_name))
        .find_map(|context| {
            graph
                .find_node(
                    "ExecutionContext",
                    &format!("{}.{}", component.name, context.name),
                )
                .cloned()
        })
}

fn resolve_system_component_scheduler_task(
    graph: &Graph,
    component: &AilSystemComponent,
    task_name: &str,
) -> Option<crate::core_model::Node> {
    component
        .scheduler_tasks
        .iter()
        .filter(|task| task.task_name.eq_ignore_ascii_case(task_name))
        .find_map(|task| {
            graph
                .find_node(
                    "SchedulerTask",
                    &format!("{}.{}", component.name, task.task_name),
                )
                .cloned()
        })
}

fn system_capability_resource_reference(capability: &str) -> Option<String> {
    let capability = trim_sentence(capability);
    for verb in [
        "access ",
        "read ",
        "write ",
        "use ",
        "configure ",
        "reset ",
        "map ",
        "unmap ",
        "allocate ",
        "free ",
        "pin ",
        "unpin ",
        "release ",
        "move ",
    ] {
        if let Some(resource) = capability.strip_prefix(verb) {
            let resource = resource.trim();
            return (!resource.is_empty()).then(|| resource.to_string());
        }
    }
    None
}

fn system_effect_resource_reference(effect: &str) -> Option<String> {
    let effect = trim_sentence(effect);
    for verb in [
        "read ",
        "write ",
        "access ",
        "release ",
        "map ",
        "unmap ",
        "allocate ",
        "free ",
        "pin ",
        "unpin ",
        "reset ",
        "configure ",
        "move ",
    ] {
        if let Some(resource) = effect.strip_prefix(verb) {
            let resource = resource.trim();
            return (!resource.is_empty()).then(|| resource.to_string());
        }
    }
    None
}

fn find_referenced_field(
    graph: &Graph,
    document: &AilDocument,
    text: &str,
) -> Option<crate::core_model::Node> {
    let text = text.to_ascii_lowercase();
    let mut candidates = Vec::new();
    for thing in document.things.values() {
        for field in thing.fields.values() {
            let field_text = field.name.to_ascii_lowercase();
            if text.contains(&field_text) {
                candidates.push(format!("{}.{}", thing.name, field.name));
            }
        }
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.len()));
    candidates
        .into_iter()
        .find_map(|candidate| graph.find_node("Field", &candidate).cloned())
}

fn has_outgoing_edge(graph: &Graph, kind: &str, source_id: &str) -> bool {
    graph
        .edges
        .iter()
        .any(|edge| edge.kind == kind && edge.source == source_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListContext {
    Users,
    Views,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolSection {
    Requirements,
    Inputs,
    Outputs,
    Capabilities,
    Protections,
    Permissions,
    Approvals,
    Traces,
    Guarantees,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompilerPassSection {
    Inputs,
    Outputs,
    Body,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SystemSection {
    Resources,
    Ownership,
    Borrowing,
    MutableBorrowing,
    Regions,
    Layouts,
    Allocations,
    LockGuards,
    ExecutionContexts,
    InterruptPriorities,
    InterruptMasks,
    SchedulerTasks,
    SchedulerTaskPriorities,
    SchedulerTaskTimings,
    Capabilities,
    Effects,
    Traces,
    Guarantees,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ContinuationTarget {
    ApplicationPurpose,
    CompilerPassPurpose(String),
    FailureCondition(String),
}

fn is_structural_line(line: &str) -> bool {
    line == "The application has these users:"
        || line == "The application shows:"
        || parse_application_line(line).is_some()
        || parse_thing_header(line).is_some()
        || parse_tool_header(line).is_some()
        || parse_tool_section(line).is_some()
        || parse_compiler_pass_header(line).is_some()
        || parse_compiler_pass_section(line).is_some()
        || parse_system_component_header(line).is_some()
        || parse_system_section(line).is_some()
        || parse_action_header(line).is_some()
        || parse_when_line(line).is_some()
        || parse_failure_header(line).is_some()
}

fn append_continuation(document: &mut AilDocument, target: &ContinuationTarget, line: &str) {
    let fragment = trim_sentence(line.trim_end_matches(':'));
    match target {
        ContinuationTarget::ApplicationPurpose => {
            append_words(&mut document.application.purpose, &fragment);
        }
        ContinuationTarget::CompilerPassPurpose(pass_name) => {
            if let Some(pass) = document.compiler_passes.get_mut(pass_name) {
                append_words(&mut pass.purpose, &fragment);
            }
        }
        ContinuationTarget::FailureCondition(failure_name) => {
            if let Some(failure) = document.failures.get_mut(failure_name) {
                append_words(&mut failure.condition, &fragment);
            }
        }
    }
}

fn append_words(target: &mut String, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push(' ');
    }
    target.push_str(fragment);
}
