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
    pub capability_grants: Vec<AilCapabilityGrant>,
    pub conformance: String,
    pub prompt_pack: Option<String>,
    pub registry: Option<String>,
    pub target_support: BTreeMap<String, String>,
    pub schema_version: Option<String>,
    pub safety_level: Option<String>,
    pub base_llm_endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilImportSpec {
    pub path: String,
    pub version: Option<String>,
    pub alias: String,
    pub resolved_package: Option<String>,
    pub registry_identity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilCapabilityGrant {
    pub package: String,
    pub capability: String,
    pub effects: Vec<String>,
    pub approvals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilLoadedImport {
    pub spec: AilImportSpec,
    pub package: Box<AilPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AilRegistryEntry {
    package: String,
    version: String,
    identity: String,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilDocument {
    pub application: AilApplication,
    pub things: BTreeMap<String, AilThing>,
    pub tools: BTreeMap<String, AilTool>,
    pub compiler_passes: BTreeMap<String, AilCompilerPass>,
    pub system_components: BTreeMap<String, AilSystemComponent>,
    pub functions: BTreeMap<String, AilFunction>,
    pub types: BTreeMap<String, AilType>,
    pub routes: BTreeMap<String, AilRoute>,
    pub forms: BTreeMap<String, AilForm>,
    pub dashboards: BTreeMap<String, AilDashboard>,
    pub workflows: BTreeMap<String, AilWorkflow>,
    pub external_bindings: BTreeMap<String, AilExternalBinding>,
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
pub struct AilFunction {
    pub name: String,
    pub label: String,
    pub inputs: BTreeMap<String, AilFunctionValue>,
    pub outputs: BTreeMap<String, AilFunctionValue>,
    pub branches: Vec<String>,
    pub calls: Vec<AilFunctionCall>,
    pub termination_bounds: Vec<String>,
    pub termination_measures: Vec<String>,
    pub returns: Vec<String>,
    pub traces: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilFunctionValue {
    pub name: String,
    pub type_name: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilFunctionCall {
    pub text: String,
    pub target: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilType {
    pub name: String,
    pub label: String,
    pub variants: BTreeMap<String, AilVariant>,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilVariant {
    pub name: String,
    pub label: String,
    pub fields: BTreeMap<String, AilVariantField>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilVariantField {
    pub name: String,
    pub type_name: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilExternalBinding {
    pub name: String,
    pub library: String,
    pub symbol: String,
    pub binding_kind: String,
    pub calling_convention: String,
    pub inputs: BTreeMap<String, AilExternalBindingValue>,
    pub outputs: BTreeMap<String, AilExternalBindingValue>,
    pub status_maps: Vec<AilExternalStatusMap>,
    pub capabilities: Vec<String>,
    pub traces: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilExternalBindingValue {
    pub name: String,
    pub type_name: String,
    pub ownership: String,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilExternalStatusMap {
    pub code: String,
    pub target: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilRoute {
    pub name: String,
    pub label: String,
    pub path: String,
    pub reads: Vec<String>,
    pub permissions: Vec<String>,
    pub traces: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilForm {
    pub name: String,
    pub label: String,
    pub action: Option<String>,
    pub fields: BTreeMap<String, AilFormField>,
    pub validations: Vec<String>,
    pub failure_traces: Vec<String>,
    pub confirmations: Vec<String>,
    pub accessibility: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilFormField {
    pub name: String,
    pub type_name: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilDashboard {
    pub name: String,
    pub label: String,
    pub reads: Vec<String>,
    pub permissions: Vec<String>,
    pub filters: Vec<String>,
    pub traces: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AilWorkflow {
    pub name: String,
    pub label: String,
    pub steps: Vec<String>,
    pub blocks: Vec<AilWorkflowBlock>,
    pub traces: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilWorkflowBlock {
    pub blocked_step: String,
    pub prerequisite_step: String,
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
    pub calls: Vec<String>,
    pub repeated_calls: Vec<AilRepeatedActionCall>,
    pub failures: Vec<String>,
    pub guarantees: Vec<String>,
    pub traces: Vec<String>,
    pub secret_protections: Vec<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AilRepeatedActionCall {
    pub target: String,
    pub count: usize,
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
    pub capability_grants: Vec<AilCapabilityGrant>,
    pub target_support: BTreeMap<String, String>,
    pub external_bindings_metadata_present: bool,
    pub external_bindings: BTreeMap<String, AilExternalBinding>,
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
    AddAction(Box<AilAction>),
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
    let mut metadata = parse_package_metadata(&metadata_text)?;
    let spec_path = root.join(&metadata.entry);
    let spec_text = fs::read_to_string(&spec_path)
        .map_err(|error| format!("failed to read {}: {error}", spec_path.display()))?;
    let registry_entries = load_package_registry_entries(&root, &metadata)?;
    let mut imports = Vec::new();
    for import_index in 0..metadata.imports.len() {
        let import = metadata.imports[import_index].clone();
        let registry_match = resolve_registry_import(&import, registry_entries.as_slice())?;
        let import_root = registry_match
            .as_ref()
            .map(|entry| {
                let path = PathBuf::from(&entry.path);
                if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                }
            })
            .unwrap_or_else(|| root.join(&import.path));
        let package = load_ail_package_dir_inner(&import_root, stack)?;
        if let Some(required_version) = &import.version
            && !import_version_requirement_matches(required_version, &package.metadata.version)?
        {
            return Err(format!(
                "AIL import {} as {} requires version {}, but package {} is version {}",
                import.path,
                import.alias,
                required_version,
                package.metadata.name,
                package.metadata.version
            ));
        }
        if let Some(registry_entry) = &registry_match {
            if registry_entry.version != package.metadata.version
                || registry_entry.package != package.metadata.name
            {
                return Err(format!(
                    "AIL registry identity {} resolves {}@{} but loaded package {} is {}@{}",
                    registry_entry.identity,
                    registry_entry.package,
                    registry_entry.version,
                    import.path,
                    package.metadata.name,
                    package.metadata.version
                ));
            }
            metadata.imports[import_index].registry_identity =
                Some(registry_entry.identity.clone());
        }
        metadata.imports[import_index].resolved_package = Some(package.metadata.name.clone());
        imports.push(AilLoadedImport {
            spec: metadata.imports[import_index].clone(),
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

fn load_package_registry_entries(
    root: &Path,
    metadata: &AilPackageMetadata,
) -> Result<Vec<AilRegistryEntry>, String> {
    let Some(registry_path) = metadata.registry.as_deref() else {
        return Ok(Vec::new());
    };
    let registry_path = root.join(registry_path);
    let registry_text = fs::read_to_string(&registry_path).map_err(|error| {
        format!(
            "failed to read AIL registry index {}: {error}",
            registry_path.display()
        )
    })?;
    parse_ail_registry_entries(&registry_text)
}

fn parse_ail_registry_entries(text: &str) -> Result<Vec<AilRegistryEntry>, String> {
    let mut entries = Vec::new();
    let mut fields = BTreeMap::<String, String>::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key == "package" && fields.contains_key("package") {
            entries.push(registry_entry_from_fields(&fields)?);
            fields.clear();
        }
        fields.insert(key.to_string(), value.to_string());
    }
    if !fields.is_empty() {
        entries.push(registry_entry_from_fields(&fields)?);
    }
    Ok(entries)
}

fn registry_entry_from_fields(
    fields: &BTreeMap<String, String>,
) -> Result<AilRegistryEntry, String> {
    let field = |name: &str| {
        fields
            .get(name)
            .cloned()
            .ok_or_else(|| format!("AIL registry entry missing {name}"))
    };
    Ok(AilRegistryEntry {
        package: field("package")?,
        version: field("version")?,
        identity: field("identity")?,
        path: field("path")?,
    })
}

fn resolve_registry_import(
    import: &AilImportSpec,
    entries: &[AilRegistryEntry],
) -> Result<Option<AilRegistryEntry>, String> {
    if entries.is_empty() || looks_like_local_import_path(&import.path) {
        return Ok(None);
    }
    let matches = entries
        .iter()
        .filter(|entry| {
            entry.package == import.path
                && import.version.as_ref().is_none_or(|requirement| {
                    import_version_requirement_matches(requirement, &entry.version).unwrap_or(false)
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entry] => Ok(Some(entry.clone())),
        [] => Err(format!(
            "AIL registry import {} as {} was not found in registry index",
            import.path, import.alias
        )),
        _ => Err(format!(
            "AIL registry import {} as {} is ambiguous in registry index",
            import.path, import.alias
        )),
    }
}

fn looks_like_local_import_path(path: &str) -> bool {
    path.starts_with('.') || path.starts_with('/') || path.contains('\\')
}

pub fn parse_ail_package_document(package: &AilPackage) -> Result<AilDocument, String> {
    parse_ail_package_spec_text(package, &package.spec_text)
}

pub fn render_ail_package_dependency_report(package: &AilPackage) -> Result<String, String> {
    let mut lines = vec![
        "AIL-Package-Dependency-Report:".to_string(),
        format!(
            "root-package {} {}",
            package.metadata.name, package.metadata.version
        ),
        format!("root-path {}", package.root.display()),
        format!("root-package-hash {}", ail_package_source_hash(package)?),
    ];
    render_ail_package_dependency_imports(package, &mut lines)?;
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_ail_package_dependency_imports(
    package: &AilPackage,
    lines: &mut Vec<String>,
) -> Result<(), String> {
    for import in &package.imports {
        let requirement = import.spec.version.as_deref().unwrap_or("none");
        let registry_identity = import
            .spec
            .registry_identity
            .as_deref()
            .map(|identity| format!(" registry-identity={identity}"))
            .unwrap_or_default();
        lines.push(format!(
            "resolved-import {} path={} requirement={} name={} version={} source-path={} package-hash={}{}",
            import.spec.alias,
            import.spec.path,
            requirement,
            import.package.metadata.name,
            import.package.metadata.version,
            import.package.root.display(),
            ail_package_source_hash(&import.package)?,
            registry_identity
        ));
        if import.package.metadata.capability_grants.is_empty() {
            lines.push(format!("capability-grants {} none", import.spec.alias));
            lines.push(format!(
                "imported-effect-classes {} none",
                import.spec.alias
            ));
        } else {
            let mut effect_classes = BTreeSet::new();
            for grant in &import.package.metadata.capability_grants {
                for effect in &grant.effects {
                    effect_classes.insert(effect.clone());
                }
                let effects = render_dependency_report_list(&grant.effects);
                let approvals = render_dependency_report_list(&grant.approvals);
                lines.push(format!(
                    "capability-grant package={} capability={} effects={} approvals={}",
                    grant.package, grant.capability, effects, approvals
                ));
            }
            let effect_classes = effect_classes.into_iter().collect::<Vec<_>>();
            lines.push(format!(
                "imported-effect-classes {} {}",
                import.spec.alias,
                render_dependency_report_list(&effect_classes)
            ));
        }
        render_ail_package_dependency_imports(&import.package, lines)?;
    }
    Ok(())
}

fn ail_package_source_hash(package: &AilPackage) -> Result<String, String> {
    let manifest_path = package.root.join("ail-package.md");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    Ok(format!(
        "ail-package:{}",
        ail_text_fingerprint(&format!(
            "ail-package.md:\n{}\n{}:\n{}",
            manifest_text, package.metadata.entry, package.spec_text
        ))
    ))
}

fn render_dependency_report_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join("|")
    }
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
                changes.push(AilPatchChange::AddAction(Box::new(action)));
            }
            section = Some("target");
            continue;
        }
        if line == "change:" {
            if let Some(action) = current_action.take() {
                changes.push(AilPatchChange::AddAction(Box::new(action)));
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
                        changes.push(AilPatchChange::AddAction(Box::new(action)));
                    }
                    changes.push(field);
                    continue;
                }
                if let Some(view) = line.strip_prefix("add view ") {
                    if let Some(action) = current_action.take() {
                        changes.push(AilPatchChange::AddAction(Box::new(action)));
                    }
                    changes.push(AilPatchChange::AddView(view.trim().to_string()));
                    continue;
                }
                if let Some(label) = line.strip_prefix("add action ") {
                    if let Some(action) = current_action.take() {
                        changes.push(AilPatchChange::AddAction(Box::new(action)));
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
        changes.push(AilPatchChange::AddAction(Box::new(action)));
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
                document
                    .actions
                    .insert(action.name.clone(), action.as_ref().clone());
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
    if root.contains_key("package") {
        let package_name = required_json_string_for(root, "package", "AIL-Core patch")?;
        if package_name != core.package.name {
            return Err(format!(
                "AIL-Core patch package mismatch: expected {}, got {package_name}",
                core.package.name
            ));
        }
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
            "remove_node" => apply_ail_core_patch_remove_node(&mut patched, op)?,
            "add_edge" => apply_ail_core_patch_add_edge(&mut patched, op)?,
            "remove_edge" => apply_ail_core_patch_remove_edge(&mut patched, op)?,
            "declare_provenance" => apply_ail_core_patch_declare_provenance(&mut patched, op)?,
            "replace_edge_attributes" => {
                apply_ail_core_patch_replace_edge_attributes(&mut patched, op)?
            }
            "replace_node_attributes" => {
                apply_ail_core_patch_replace_node_attributes(&mut patched, op)?
            }
            op_name => return Err(format!("unsupported AIL-Core patch op '{op_name}'")),
        }
    }
    let diagnostics = check_ail_core(&patched);
    if !diagnostics.is_empty() {
        return Err(format!(
            "AIL-Core patch result failed checker: {}",
            diagnostics.join("; ")
        ));
    }
    Ok(patched)
}

pub fn apply_ail_flow_edit_text(core: &AilCore, edit_text: &str) -> Result<AilCore, String> {
    let patch_text = render_ail_core_patch_from_flow_edit_text(core, edit_text)?;
    apply_ail_core_patch_text(core, &patch_text)
}

pub fn render_ail_core_patch_from_flow_edit_text(
    core: &AilCore,
    edit_text: &str,
) -> Result<String, String> {
    let mut parser = AilJsonParser::new(edit_text);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if !parser.is_finished() {
        return Err("unexpected trailing content in AIL-Flow edit artifact".to_string());
    }
    let root = value
        .as_object()
        .ok_or_else(|| "AIL-Flow edit artifact must be a JSON object".to_string())?;
    let schema = required_json_string_for(root, "schema", "AIL-Flow edit")?;
    if schema != "ail-flow.edit.v0" {
        return Err(format!("expected ail-flow.edit.v0 edit, got '{schema}'"));
    }
    if root.contains_key("package") {
        let package_name = required_json_string_for(root, "package", "AIL-Flow edit")?;
        if package_name != core.package.name {
            return Err(format!(
                "AIL-Flow edit package mismatch: expected {}, got {package_name}",
                core.package.name
            ));
        }
    }
    let base_hash = required_json_string_for(root, "base_hash", "AIL-Flow edit")?;
    let actual_hash = ail_core_hash(core);
    if base_hash != actual_hash {
        return Err(format!(
            "AIL-Flow edit base_hash mismatch: expected {actual_hash}, got {base_hash}"
        ));
    }
    let source_view = required_json_string_for(root, "source_view", "AIL-Flow edit")?;
    let mut ops = Vec::new();
    for edit_value in required_json_array_for(root, "edits", "AIL-Flow edit")? {
        let edit = edit_value
            .as_object()
            .ok_or_else(|| "AIL-Flow edit entry must be an object".to_string())?;
        match required_json_string_for(edit, "op", "AIL-Flow edit entry")? {
            "ActionCard.rename" => {
                ops.push(render_action_card_rename_core_patch_op(core, edit)?);
            }
            "ActionCard.addRequirement" => {
                ops.extend(render_action_card_add_requirement_core_patch_ops(
                    core, edit,
                )?);
            }
            "DataTable.addField" => {
                ops.extend(render_data_table_add_field_core_patch_ops(core, edit)?);
            }
            op_name => return Err(format!("unsupported AIL-Flow edit op '{op_name}'")),
        }
    }
    Ok(format!(
        concat!(
            "{{\n",
            "  \"schema\": \"ail-core.patch.v0\",\n",
            "  \"package\": {},\n",
            "  \"base_hash\": {},\n",
            "  \"source_view\": {},\n",
            "  \"ops\": [\n",
            "{}\n",
            "  ]\n",
            "}}"
        ),
        json_string(&core.package.name),
        json_string(base_hash),
        json_string(source_view),
        ops.join(",\n")
    ))
}

fn render_action_card_add_requirement_core_patch_ops(
    core: &AilCore,
    edit: &BTreeMap<String, AilJsonValue>,
) -> Result<Vec<String>, String> {
    let target = required_json_string_for(edit, "target", "AIL-Flow ActionCard.addRequirement")?;
    let Some(target_node) = find_core_patch_node(core, target) else {
        return Err(format!(
            "AIL-Flow ActionCard.addRequirement references unknown target '{target}'"
        ));
    };
    if target_node.kind != "Action" {
        return Err(format!(
            "AIL-Flow ActionCard.addRequirement target must be an Action, got {}",
            core_node_label(&target_node)
        ));
    }
    let requirement = trim_sentence(required_json_string_for(
        edit,
        "requirement",
        "AIL-Flow ActionCard.addRequirement",
    )?);
    if requirement.is_empty() {
        return Err("AIL-Flow ActionCard.addRequirement requirement must not be empty".to_string());
    }
    let mut provenance =
        optional_json_string_array(edit, "provenance", "AIL-Flow ActionCard.addRequirement")?;
    if provenance.is_empty() {
        provenance.push(format!(
            "flow:ActionCard:{}.requirement:{}",
            target_node.name, requirement
        ));
    }
    let provenance_array = render_json_array(provenance.clone());
    let rule_label = format!("Rule:{requirement}");
    let mut ops = Vec::new();
    if core.graph.find_node("Rule", &requirement).is_some() {
        ops.push(format!(
            concat!(
                "    {{\n",
                "      \"op\": \"declare_provenance\",\n",
                "      \"target\": {},\n",
                "      \"provenance\": {}\n",
                "    }}"
            ),
            json_string(&rule_label),
            provenance_array
        ));
    } else {
        ops.push(format!(
            concat!(
                "    {{\n",
                "      \"op\": \"add_node\",\n",
                "      \"kind\": \"Rule\",\n",
                "      \"name\": {},\n",
                "      \"provenance\": {}\n",
                "    }}"
            ),
            json_string(&requirement),
            provenance_array
        ));
    }
    ops.push(format!(
        concat!(
            "    {{\n",
            "      \"op\": \"add_edge\",\n",
            "      \"kind\": \"requires\",\n",
            "      \"source\": {},\n",
            "      \"target\": {},\n",
            "      \"provenance\": {}\n",
            "    }}"
        ),
        json_string(&core_node_label(&target_node)),
        json_string(&rule_label),
        render_json_array(provenance)
    ));
    Ok(ops)
}

fn render_data_table_add_field_core_patch_ops(
    core: &AilCore,
    edit: &BTreeMap<String, AilJsonValue>,
) -> Result<Vec<String>, String> {
    let target = required_json_string_for(edit, "target", "AIL-Flow DataTable.addField")?;
    let Some(target_node) = find_core_patch_node(core, target) else {
        return Err(format!(
            "AIL-Flow DataTable.addField references unknown target '{target}'"
        ));
    };
    if target_node.kind != "Thing" {
        return Err(format!(
            "AIL-Flow DataTable.addField target must be a Thing, got {}",
            core_node_label(&target_node)
        ));
    }
    let field_name = trim_sentence(required_json_string_for(
        edit,
        "name",
        "AIL-Flow DataTable.addField",
    )?);
    if field_name.is_empty() {
        return Err("AIL-Flow DataTable.addField name must not be empty".to_string());
    }
    if field_name.contains('.') {
        return Err(
            "AIL-Flow DataTable.addField name must be local to the target Thing".to_string(),
        );
    }
    let type_name = trim_sentence(required_json_string_for(
        edit,
        "type",
        "AIL-Flow DataTable.addField",
    )?);
    if type_name.is_empty() {
        return Err("AIL-Flow DataTable.addField type must not be empty".to_string());
    }
    let secret = match optional_json_string(edit, "secret") {
        Some("true") => true,
        Some("false") => false,
        Some(value) => {
            return Err(format!(
                "AIL-Flow DataTable.addField secret must be 'true' or 'false', got '{value}'"
            ));
        }
        None => type_contains_secret(&type_name),
    };
    let mut provenance =
        optional_json_string_array(edit, "provenance", "AIL-Flow DataTable.addField")?;
    if provenance.is_empty() {
        provenance.push(format!(
            "flow:DataTable:{}.field:{}",
            target_node.name, field_name
        ));
    }
    let provenance_array = render_json_array(provenance.clone());
    let field_full_name = format!("{}.{}", target_node.name, field_name);
    let field_label = format!("Field:{field_full_name}");
    let mut ops = vec![
        format!(
            concat!(
                "    {{\n",
                "      \"op\": \"add_node\",\n",
                "      \"kind\": \"Field\",\n",
                "      \"name\": {},\n",
                "      \"type\": {},\n",
                "      \"attributes\": {{\n",
                "        \"secret\": {}\n",
                "      }},\n",
                "      \"provenance\": {}\n",
                "    }}"
            ),
            json_string(&field_full_name),
            json_string(&type_name),
            json_string(if secret { "true" } else { "false" }),
            provenance_array
        ),
        format!(
            concat!(
                "    {{\n",
                "      \"op\": \"add_edge\",\n",
                "      \"kind\": \"has_field\",\n",
                "      \"source\": {},\n",
                "      \"target\": {},\n",
                "      \"provenance\": {}\n",
                "    }}"
            ),
            json_string(&core_node_label(&target_node)),
            json_string(&field_label),
            render_json_array(provenance.clone())
        ),
    ];
    if secret {
        let secret_label = format!("Secret:{field_full_name}");
        ops.push(format!(
            concat!(
                "    {{\n",
                "      \"op\": \"add_node\",\n",
                "      \"kind\": \"Secret\",\n",
                "      \"name\": {},\n",
                "      \"provenance\": {}\n",
                "    }}"
            ),
            json_string(&field_full_name),
            render_json_array(provenance.clone())
        ));
        ops.push(format!(
            concat!(
                "    {{\n",
                "      \"op\": \"add_edge\",\n",
                "      \"kind\": \"protects_secret\",\n",
                "      \"source\": {},\n",
                "      \"target\": {},\n",
                "      \"provenance\": {}\n",
                "    }}"
            ),
            json_string(&secret_label),
            json_string(&field_label),
            render_json_array(provenance)
        ));
    }
    Ok(ops)
}

fn render_action_card_rename_core_patch_op(
    core: &AilCore,
    edit: &BTreeMap<String, AilJsonValue>,
) -> Result<String, String> {
    let target = required_json_string_for(edit, "target", "AIL-Flow ActionCard.rename")?;
    let Some(target_node) = find_core_patch_node(core, target) else {
        return Err(format!(
            "AIL-Flow ActionCard.rename references unknown target '{target}'"
        ));
    };
    if target_node.kind != "Action" {
        return Err(format!(
            "AIL-Flow ActionCard.rename target must be an Action, got {}",
            core_node_label(&target_node)
        ));
    }
    let label = required_json_string_for(edit, "label", "AIL-Flow ActionCard.rename")?;
    let provenance = optional_json_string_array(edit, "provenance", "AIL-Flow ActionCard.rename")?;
    let provenance_field = if provenance.is_empty() {
        String::new()
    } else {
        format!(",\n      \"provenance\": {}", render_json_array(provenance))
    };
    Ok(format!(
        concat!(
            "    {{\n",
            "      \"op\": \"replace_node_attributes\",\n",
            "      \"target\": {},\n",
            "      \"attributes\": {{\n",
            "        \"label\": {}\n",
            "      }}{}\n",
            "    }}"
        ),
        json_string(&core_node_label(&target_node)),
        json_string(label),
        provenance_field
    ))
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
    let provenance = optional_json_string_array(op, "provenance", "AIL-Core patch add_node")?;
    if core
        .graph
        .nodes
        .iter()
        .any(|node| node.kind == kind && node.name == name)
    {
        return Err(format!(
            "AIL-Core patch add_node refuses to add existing node {kind}:{name}"
        ));
    }
    let node = core
        .graph
        .add_node(kind.to_string(), name.to_string(), type_name, attributes);
    for provenance in provenance {
        attach_provenance(&mut core.graph, &node, provenance);
    }
    Ok(())
}

fn apply_ail_core_patch_remove_node(
    core: &mut AilCore,
    op: &BTreeMap<String, AilJsonValue>,
) -> Result<(), String> {
    let target_label = required_json_string_for(op, "target", "AIL-Core patch remove_node")?;
    let target = find_core_patch_node(core, target_label).ok_or_else(|| {
        format!("AIL-Core patch remove_node references unknown target '{target_label}'")
    })?;
    let incident_edges = core_patch_incident_edge_descriptions(core, &target.id);
    if !incident_edges.is_empty() {
        return Err(format!(
            "AIL-Core patch remove_node refuses to remove {target_label} because it has incident edges; remove edges first: {}",
            incident_edges.join(", ")
        ));
    }
    core.graph.nodes.retain(|node| node.id != target.id);
    Ok(())
}

fn apply_ail_core_patch_declare_provenance(
    core: &mut AilCore,
    op: &BTreeMap<String, AilJsonValue>,
) -> Result<(), String> {
    let target_label = required_json_string_for(op, "target", "AIL-Core patch declare_provenance")?;
    let target = find_core_patch_node(core, target_label).ok_or_else(|| {
        format!("AIL-Core patch declare_provenance references unknown target '{target_label}'")
    })?;
    let provenance =
        optional_json_string_array(op, "provenance", "AIL-Core patch declare_provenance")?;
    if provenance.is_empty() {
        return Err("AIL-Core patch declare_provenance must provide provenance".to_string());
    }
    for entry in provenance {
        attach_provenance(&mut core.graph, &target, entry);
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
    if core
        .graph
        .edges
        .iter()
        .any(|edge| edge.kind == kind && edge.source == source.id && edge.target == target.id)
    {
        return Err(format!(
            "AIL-Core patch add_edge refuses to add existing edge {kind} {source_label} -> {target_label}"
        ));
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
    let ordinal = core.graph.edges[edge_index].ordinal;
    let mut edge = Edge::new(kind.to_string(), &source, &target, attributes);
    edge.ordinal = ordinal;
    core.graph.edges[edge_index] = edge;
    Ok(())
}

fn core_patch_incident_edge_descriptions(core: &AilCore, target_id: &str) -> Vec<String> {
    let node_by_id = graph_node_by_id(core);
    let mut descriptions = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.source == target_id || edge.target == target_id)
        .map(|edge| {
            let source = node_by_id
                .get(&edge.source)
                .map(core_node_label)
                .unwrap_or_else(|| edge.source.clone());
            let target = node_by_id
                .get(&edge.target)
                .map(core_node_label)
                .unwrap_or_else(|| edge.target.clone());
            format!("{} {} -> {}", edge.kind, source, target)
        })
        .collect::<Vec<_>>();
    descriptions.sort();
    descriptions
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
            let ordinal = edge.ordinal;
            let mut resolved_edge =
                Edge::new(edge.kind.clone(), source, target, edge.attributes.clone());
            resolved_edge.ordinal = ordinal;
            *edge = resolved_edge;
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
        functions: BTreeMap::new(),
        types: BTreeMap::new(),
        routes: BTreeMap::new(),
        forms: BTreeMap::new(),
        dashboards: BTreeMap::new(),
        workflows: BTreeMap::new(),
        external_bindings: BTreeMap::new(),
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
    let mut current_function: Option<String> = None;
    let mut current_function_section: Option<FunctionSection> = None;
    let mut current_type: Option<String> = None;
    let mut current_type_section: Option<TypeSection> = None;
    let mut current_route: Option<String> = None;
    let mut current_route_section: Option<RouteSection> = None;
    let mut current_form: Option<String> = None;
    let mut current_form_section: Option<FormSection> = None;
    let mut current_dashboard: Option<String> = None;
    let mut current_dashboard_section: Option<DashboardSection> = None;
    let mut current_workflow: Option<String> = None;
    let mut current_workflow_section: Option<WorkflowSection> = None;
    let mut current_c_library: Option<String> = None;
    let mut current_external_binding: Option<String> = None;
    let mut current_external_binding_section: Option<ExternalBindingSection> = None;
    let mut current_action: Option<String> = None;
    let mut current_failure: Option<String> = None;
    let mut current_list: Option<ListContext> = None;
    let mut continuation: Option<ContinuationTarget> = None;
    let mut action_header_waiting_for_when = false;

    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let raw_line = raw_line.trim();
        if raw_line.is_empty() {
            continue;
        }
        let line = if raw_line.starts_with('#') {
            let heading = raw_line.trim_start_matches('#').trim();
            if heading.is_empty() {
                continue;
            }
            heading
        } else {
            raw_line
        };
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            continue;
        }
        if let Some(library) = parse_c_library_header(line) {
            current_c_library = Some(library);
            current_external_binding = None;
            current_external_binding_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some((library, symbol)) =
            parse_external_function_import(line, current_c_library.as_deref())
        {
            let name = format!("{library}.{symbol}");
            document
                .external_bindings
                .entry(name.clone())
                .or_insert_with(|| AilExternalBinding {
                    name: name.clone(),
                    library,
                    symbol,
                    binding_kind: "CFunction".to_string(),
                    calling_convention: "cdecl".to_string(),
                    provenance: format!("external_binding:{name}"),
                    ..AilExternalBinding::default()
                });
            current_external_binding = Some(name);
            current_external_binding_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some((binding_name, section)) = parse_external_binding_section(&document, line) {
            current_external_binding = Some(binding_name);
            current_external_binding_section = Some(section);
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some((binding_name, trace)) = parse_external_trace_event_line(&document, line) {
            if let Some(binding) = document.external_bindings.get_mut(&binding_name) {
                binding.traces.push(trace);
            }
            current_external_binding = Some(binding_name);
            current_external_binding_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(label) = parse_type_header(line) {
            let name = label.clone();
            document
                .types
                .entry(name.clone())
                .or_insert_with(|| AilType {
                    name: name.clone(),
                    label,
                    provenance: format!("type:{name}"),
                    ..AilType::default()
                });
            current_type = Some(name);
            current_type_section = None;
            current_external_binding = None;
            current_external_binding_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(type_name) = parse_type_variants_header(&document, line) {
            current_type = Some(type_name);
            current_type_section = Some(TypeSection::Variants);
            current_external_binding = None;
            current_external_binding_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(label) = parse_route_header(line) {
            let name = action_name_from_label(&label);
            document
                .routes
                .entry(name.clone())
                .or_insert_with(|| AilRoute {
                    name: name.clone(),
                    label,
                    provenance: format!("route:{name}"),
                    ..AilRoute::default()
                });
            current_route = Some(name);
            current_route_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(section) = parse_route_section(line)
            && current_route.is_some()
        {
            current_route_section = Some(section);
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(label) = parse_form_header(line) {
            let name = format!("{}Form", action_name_from_label(&label));
            document
                .forms
                .entry(name.clone())
                .or_insert_with(|| AilForm {
                    name: name.clone(),
                    label,
                    provenance: format!("form:{name}"),
                    ..AilForm::default()
                });
            current_form = Some(name);
            current_form_section = None;
            current_route = None;
            current_route_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(section) = parse_form_section(line)
            && current_form.is_some()
        {
            current_form_section = Some(section);
            current_route = None;
            current_route_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(label) = parse_dashboard_header(line) {
            let name = action_name_from_label(&label);
            document
                .dashboards
                .entry(name.clone())
                .or_insert_with(|| AilDashboard {
                    name: name.clone(),
                    label,
                    provenance: format!("dashboard:{name}"),
                    ..AilDashboard::default()
                });
            current_dashboard = Some(name);
            current_dashboard_section = None;
            current_form = None;
            current_form_section = None;
            current_route = None;
            current_route_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(section) = parse_dashboard_section(line)
            && current_dashboard.is_some()
        {
            current_dashboard_section = Some(section);
            current_form = None;
            current_form_section = None;
            current_route = None;
            current_route_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(label) = parse_workflow_header(line) {
            let name = action_name_from_label(&label);
            document
                .workflows
                .entry(name.clone())
                .or_insert_with(|| AilWorkflow {
                    name: name.clone(),
                    label,
                    provenance: format!("workflow:{name}"),
                    ..AilWorkflow::default()
                });
            current_workflow = Some(name);
            current_workflow_section = None;
            current_dashboard = None;
            current_dashboard_section = None;
            current_form = None;
            current_form_section = None;
            current_route = None;
            current_route_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(section) = parse_workflow_section(line)
            && current_workflow.is_some()
        {
            current_workflow_section = Some(section);
            current_dashboard = None;
            current_dashboard_section = None;
            current_form = None;
            current_form_section = None;
            current_route = None;
            current_route_section = None;
            current_thing = None;
            current_tool = None;
            current_tool_section = None;
            current_compiler_pass = None;
            current_compiler_pass_section = None;
            current_system_component = None;
            current_system_section = None;
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
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
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            continue;
        }
        if let Some(thing_name) = parse_markdown_thing_heading(line) {
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
            current_action = None;
            current_failure = None;
            current_list = None;
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(label) = parse_function_header(line) {
            let name = label.clone();
            document
                .functions
                .entry(name.clone())
                .or_insert_with(|| AilFunction {
                    name: name.clone(),
                    label,
                    provenance: format!("function:{name}"),
                    ..AilFunction::default()
                });
            current_function = Some(name);
            current_function_section = None;
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
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(section) = parse_function_section(line)
            && current_function.is_some()
        {
            current_function_section = Some(section);
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
            action_header_waiting_for_when = false;
            continue;
        }
        if let Some(function_name) = parse_function_body_header(line)
            && document.functions.contains_key(&function_name)
        {
            current_function = Some(function_name);
            current_function_section = Some(FunctionSection::Body);
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
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
            current_function = None;
            current_function_section = None;
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
            if let (Some(function_name), Some(section)) =
                (&current_function, current_function_section)
            {
                parse_function_bullet(&mut document, function_name, section, bullet, line_number)?;
                continue;
            }
            if let (Some(type_name), Some(section)) = (&current_type, current_type_section) {
                parse_type_bullet(&mut document, type_name, section, bullet, line_number)?;
                continue;
            }
            if let (Some(route_name), Some(section)) = (&current_route, current_route_section) {
                parse_route_bullet(&mut document, route_name, section, bullet, line_number)?;
                continue;
            }
            if let (Some(form_name), Some(section)) = (&current_form, current_form_section) {
                parse_form_bullet(&mut document, form_name, section, bullet, line_number)?;
                continue;
            }
            if let (Some(dashboard_name), Some(section)) =
                (&current_dashboard, current_dashboard_section)
            {
                parse_dashboard_bullet(
                    &mut document,
                    dashboard_name,
                    section,
                    bullet,
                    line_number,
                )?;
                continue;
            }
            if let (Some(workflow_name), Some(section)) =
                (&current_workflow, current_workflow_section)
            {
                parse_workflow_bullet(&mut document, workflow_name, section, bullet, line_number)?;
                continue;
            }
            if let (Some(binding_name), Some(section)) =
                (&current_external_binding, current_external_binding_section)
            {
                parse_external_binding_bullet(
                    &mut document,
                    binding_name,
                    section,
                    bullet,
                    line_number,
                )?;
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
            if parse_compact_thing_bullet(&mut document, bullet).is_some() {
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
        && document.functions.is_empty()
        && document.types.is_empty()
        && document.routes.is_empty()
        && document.forms.is_empty()
        && document.dashboards.is_empty()
        && document.workflows.is_empty()
        && document.external_bindings.is_empty()
    {
        return Err(
            "AIL-Spec missing application, tool, compiler pass, system component, function, type, route, form, dashboard, workflow, or external binding declaration"
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

    for type_decl in document.types.values() {
        let type_node = graph.add_node(
            "Type",
            &type_decl.name,
            None,
            attr(&[("label", &type_decl.label)]),
        );
        attach_provenance(&mut graph, &type_node, &type_decl.provenance);
        for variant in type_decl.variants.values() {
            let variant_node = graph.add_node(
                "Variant",
                format!("{}.{}", type_decl.name, variant.name),
                None,
                attr(&[("label", &variant.label)]),
            );
            graph.add_edge("contains", &type_node, &variant_node, BTreeMap::new());
            attach_provenance(&mut graph, &variant_node, &variant.provenance);
            for field in variant.fields.values() {
                let field_node = graph.add_node(
                    "Field",
                    format!("{}.{}.{}", type_decl.name, variant.name, field.name),
                    Some(field.type_name.clone()),
                    BTreeMap::new(),
                );
                graph.add_edge("has_field", &variant_node, &field_node, BTreeMap::new());
                attach_provenance(&mut graph, &field_node, &field.provenance);
            }
        }
    }

    for route in document.routes.values() {
        let route_node = graph.add_node(
            "Route",
            &route.name,
            None,
            attr(&[("label", &route.label), ("path", &route.path)]),
        );
        attach_provenance(&mut graph, &route_node, &route.provenance);
        for read in &route.reads {
            let value_node = graph.add_node(
                "Value",
                format!("{}.{}", route.name, read),
                None,
                BTreeMap::new(),
            );
            graph.add_edge(
                "reads",
                &route_node,
                &value_node,
                attr(&[("provenance", &format!("route:{}.read:{read}", route.name))]),
            );
            attach_provenance(
                &mut graph,
                &value_node,
                format!("route:{}.read:{read}", route.name),
            );
        }
        for permission in &route.permissions {
            let permission_node = graph.add_node("Permission", permission, None, BTreeMap::new());
            graph.add_edge("requires", &route_node, &permission_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &permission_node,
                format!("route:{}.permission:{permission}", route.name),
            );
        }
        for trace in &route.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge("records_trace", &route_node, &trace_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("route:{}.trace:{trace}", route.name),
            );
        }
    }

    for form in document.forms.values() {
        let form_node = graph.add_node("Form", &form.name, None, attr(&[("label", &form.label)]));
        attach_provenance(&mut graph, &form_node, &form.provenance);
        if let Some(action) = &form.action {
            let action_node = graph.add_node("Action", action, None, BTreeMap::new());
            graph.add_edge(
                "calls",
                &form_node,
                &action_node,
                attr(&[("provenance", &format!("form:{}.action:{action}", form.name))]),
            );
        }
        for field in form.fields.values() {
            let field_node = graph.add_node(
                "Field",
                format!("{}.{}", form.name, field.name),
                Some(field.type_name.clone()),
                BTreeMap::new(),
            );
            graph.add_edge("has_field", &form_node, &field_node, BTreeMap::new());
            attach_provenance(&mut graph, &field_node, &field.provenance);
        }
        for validation in &form.validations {
            let rule_node = graph.add_node("Rule", validation, None, BTreeMap::new());
            graph.add_edge("validates", &form_node, &rule_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &rule_node,
                format!("form:{}.validation:{validation}", form.name),
            );
        }
        for trace in &form.failure_traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge("records_trace", &form_node, &trace_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("form:{}.trace:{trace}", form.name),
            );
        }
        for confirmation in &form.confirmations {
            let confirmation_node =
                graph.add_node("Confirmation", confirmation, None, BTreeMap::new());
            graph.add_edge(
                "requires_confirmation",
                &form_node,
                &confirmation_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &confirmation_node,
                format!("form:{}.confirmation:{confirmation}", form.name),
            );
        }
        for accessibility in &form.accessibility {
            let accessibility_node =
                graph.add_node("Accessibility", accessibility, None, BTreeMap::new());
            graph.add_edge(
                "has_accessibility",
                &form_node,
                &accessibility_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &accessibility_node,
                format!("form:{}.accessibility:{accessibility}", form.name),
            );
        }
    }

    for dashboard in document.dashboards.values() {
        let dashboard_node = graph.add_node(
            "Dashboard",
            &dashboard.name,
            None,
            attr(&[("label", &dashboard.label)]),
        );
        attach_provenance(&mut graph, &dashboard_node, &dashboard.provenance);
        for read in &dashboard.reads {
            let value_node = graph.add_node(
                "Value",
                format!("{}.{}", dashboard.name, read),
                None,
                BTreeMap::new(),
            );
            graph.add_edge(
                "reads",
                &dashboard_node,
                &value_node,
                attr(&[(
                    "provenance",
                    &format!("dashboard:{}.read:{read}", dashboard.name),
                )]),
            );
            attach_provenance(
                &mut graph,
                &value_node,
                format!("dashboard:{}.read:{read}", dashboard.name),
            );
        }
        for permission in &dashboard.permissions {
            let permission_node = graph.add_node("Permission", permission, None, BTreeMap::new());
            graph.add_edge(
                "requires",
                &dashboard_node,
                &permission_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &permission_node,
                format!("dashboard:{}.permission:{permission}", dashboard.name),
            );
        }
        for filter in &dashboard.filters {
            let filter_node = graph.add_node("Filter", filter, None, BTreeMap::new());
            graph.add_edge("filters", &dashboard_node, &filter_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &filter_node,
                format!("dashboard:{}.filter:{filter}", dashboard.name),
            );
        }
        for trace in &dashboard.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge(
                "records_trace",
                &dashboard_node,
                &trace_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("dashboard:{}.trace:{trace}", dashboard.name),
            );
        }
    }

    for workflow in document.workflows.values() {
        let workflow_node = graph.add_node(
            "Workflow",
            &workflow.name,
            None,
            attr(&[("label", &workflow.label)]),
        );
        attach_provenance(&mut graph, &workflow_node, &workflow.provenance);
        for step in &workflow.steps {
            let step_node = graph.add_node(
                "Step",
                format!("{}.{}", workflow.name, step),
                None,
                attr(&[("label", step)]),
            );
            graph.add_edge("contains", &workflow_node, &step_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &step_node,
                format!("workflow:{}.step:{step}", workflow.name),
            );
        }
        for block in &workflow.blocks {
            let blocked_node = graph.add_node(
                "Step",
                format!("{}.{}", workflow.name, block.blocked_step),
                None,
                attr(&[("label", &block.blocked_step)]),
            );
            let prerequisite_node = graph.add_node(
                "Step",
                format!("{}.{}", workflow.name, block.prerequisite_step),
                None,
                attr(&[("label", &block.prerequisite_step)]),
            );
            graph.add_edge(
                "blocks_before",
                &blocked_node,
                &prerequisite_node,
                attr(&[("provenance", &block.provenance)]),
            );
        }
        for trace in &workflow.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge(
                "records_trace",
                &workflow_node,
                &trace_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("workflow:{}.trace:{trace}", workflow.name),
            );
        }
    }

    for function in document.functions.values() {
        let function_node = graph.add_node(
            "Function",
            &function.name,
            None,
            attr(&[("label", &function.label)]),
        );
        if let Some(application) = &application {
            graph.add_edge("contains", application, &function_node, BTreeMap::new());
        }
        attach_provenance(&mut graph, &function_node, &function.provenance);
        for input in function.inputs.values() {
            let input_node = graph.add_node(
                "Input",
                format!("{}.{}", function.name, input.name),
                Some(input.type_name.clone()),
                BTreeMap::new(),
            );
            graph.add_edge("has_input", &function_node, &input_node, BTreeMap::new());
            attach_provenance(&mut graph, &input_node, &input.provenance);
        }
        for output in function.outputs.values() {
            let output_node = graph.add_node(
                "Output",
                format!("{}.{}", function.name, output.name),
                Some(output.type_name.clone()),
                BTreeMap::new(),
            );
            graph.add_edge("has_output", &function_node, &output_node, BTreeMap::new());
            attach_provenance(&mut graph, &output_node, &output.provenance);
        }
        for branch in &function.branches {
            let branch_node = graph.add_node(
                "Branch",
                format!("{}.{}", function.name, branch),
                None,
                attr(&[("condition", branch)]),
            );
            graph.add_edge("contains", &function_node, &branch_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &branch_node,
                format!("function:{}.branch:{branch}", function.name),
            );
        }
        for call in &function.calls {
            let call_node = graph.add_node(
                "Call",
                format!("{}.{}", function.name, call.text),
                None,
                attr(&[("target", &call.target)]),
            );
            graph.add_edge("calls", &function_node, &call_node, BTreeMap::new());
            attach_provenance(&mut graph, &call_node, &call.provenance);
        }
        for bound in &function.termination_bounds {
            let bound_node = graph.add_node(
                "TerminationBound",
                format!("{}.{}", function.name, bound),
                None,
                attr(&[("value", bound)]),
            );
            graph.add_edge(
                "has_termination_bound",
                &function_node,
                &bound_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &bound_node,
                format!("function:{}.termination_bound:{bound}", function.name),
            );
        }
        for measure in &function.termination_measures {
            let measure_node = graph.add_node(
                "TerminationMeasure",
                format!("{}.{}", function.name, measure),
                None,
                attr(&[("value", measure)]),
            );
            graph.add_edge(
                "has_termination_measure",
                &function_node,
                &measure_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &measure_node,
                format!("function:{}.termination_measure:{measure}", function.name),
            );
        }
        for return_value in &function.returns {
            let return_node = graph.add_node(
                "Return",
                format!("{}.{}", function.name, return_value),
                None,
                attr(&[("value", return_value)]),
            );
            graph.add_edge("contains", &function_node, &return_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &return_node,
                format!("function:{}.return:{return_value}", function.name),
            );
        }
        for trace in &function.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge(
                "records_trace",
                &function_node,
                &trace_node,
                BTreeMap::new(),
            );
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("function:{}.trace:{trace}", function.name),
            );
        }
    }

    for binding in document.external_bindings.values() {
        let binding_node = graph.add_node(
            "ExternalBinding",
            &binding.name,
            None,
            attr(&[
                ("binding_kind", &binding.binding_kind),
                ("library", &binding.library),
                ("symbol", &binding.symbol),
            ]),
        );
        attach_provenance(&mut graph, &binding_node, &binding.provenance);
        let layout_node = graph.add_node(
            "Layout",
            format!("{}.signature", binding.name),
            Some(binding.calling_convention.clone()),
            BTreeMap::new(),
        );
        graph.add_edge("uses_layout", &binding_node, &layout_node, BTreeMap::new());
        attach_provenance(
            &mut graph,
            &layout_node,
            format!("external_binding:{}.layout", binding.name),
        );
        for input in binding.inputs.values() {
            let input_node = graph.add_node(
                "Input",
                format!("{}.{}", binding.name, input.name),
                Some(input.type_name.clone()),
                external_binding_value_attributes(input),
            );
            graph.add_edge("has_input", &binding_node, &input_node, BTreeMap::new());
            attach_provenance(&mut graph, &input_node, &input.provenance);
        }
        for output in binding.outputs.values() {
            let output_node = graph.add_node(
                "Output",
                format!("{}.{}", binding.name, output.name),
                Some(output.type_name.clone()),
                external_binding_value_attributes(output),
            );
            graph.add_edge("has_output", &binding_node, &output_node, BTreeMap::new());
            attach_provenance(&mut graph, &output_node, &output.provenance);
        }
        for status_map in &binding.status_maps {
            if let Some(failure_name) = external_status_map_failure(&status_map.target) {
                let failure_node = graph.add_node("Failure", failure_name, None, BTreeMap::new());
                graph.add_edge(
                    "may_fail_with",
                    &binding_node,
                    &failure_node,
                    attr(&[("code", &status_map.code)]),
                );
                attach_provenance(&mut graph, &failure_node, &status_map.provenance);
            } else {
                let status_node = graph.add_node(
                    "StatusMap",
                    format!("{}.{}", binding.name, status_map.code),
                    Some(status_map.target.clone()),
                    attr(&[("code", &status_map.code)]),
                );
                graph.add_edge(
                    "maps_status",
                    &binding_node,
                    &status_node,
                    attr(&[("code", &status_map.code)]),
                );
                attach_provenance(&mut graph, &status_node, &status_map.provenance);
            }
        }
        for capability in &binding.capabilities {
            let capability_node = graph.add_node("Capability", capability, None, BTreeMap::new());
            graph.add_edge("requires", &binding_node, &capability_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &capability_node,
                format!("external_binding:{}.capability:{capability}", binding.name),
            );
        }
        for trace in &binding.traces {
            let trace_node = graph.add_node("Trace", trace, None, BTreeMap::new());
            graph.add_edge("records_trace", &binding_node, &trace_node, BTreeMap::new());
            attach_provenance(
                &mut graph,
                &trace_node,
                format!("external_binding:{}.trace:{trace}", binding.name),
            );
        }
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
        for call in &action.calls {
            let target = resolve_action_call_target(&graph, document, call).unwrap_or_else(|| {
                graph.add_node("Effect", format!("call {call}"), None, BTreeMap::new())
            });
            graph.add_edge("calls", &action_node, &target, BTreeMap::new());
            if target.kind == "Effect" {
                attach_provenance(
                    &mut graph,
                    &target,
                    format!("action:{}.call:{call}", action.name),
                );
            }
        }
        for repeated_call in &action.repeated_calls {
            let target = resolve_action_call_target(&graph, document, &repeated_call.target)
                .unwrap_or_else(|| {
                    graph.add_node(
                        "Effect",
                        format!("repeat {}", repeated_call.target),
                        None,
                        BTreeMap::new(),
                    )
                });
            graph.add_edge(
                "repeats",
                &action_node,
                &target,
                attr(&[("count", &repeated_call.count.to_string())]),
            );
            if target.kind == "Effect" {
                attach_provenance(&mut graph, &target, repeated_call.provenance.clone());
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
    diagnostics.extend(check_core_schema_version(core));
    diagnostics.extend(check_package_safety_level(core));
    diagnostics.extend(check_package_target_support_statuses(core));
    diagnostics.extend(check_core_schema_catalog(core));
    diagnostics.extend(check_field_types(core));
    diagnostics.extend(check_requirement_reference_diagnostics(core));
    diagnostics.extend(check_requirement_field_references(core));
    diagnostics.extend(check_action_failure_declarations(core));
    diagnostics.extend(check_secret_write_protection(core));
    diagnostics.extend(check_secret_read_protection(core));
    diagnostics.extend(check_secret_internal_notes_role_requirements(core));
    diagnostics.extend(check_application_assignment_role_requirements(core));
    diagnostics.extend(check_application_overdue_time_requirements(core));
    diagnostics.extend(check_application_status_public_update(core));
    diagnostics.extend(check_application_notification_audit_requirements(core));
    diagnostics.extend(check_application_incident_lifecycle_status_requirements(
        core,
    ));
    diagnostics.extend(check_application_private_notes_public_timeline(core));
    diagnostics.extend(check_application_incident_escalation_policy_review(core));
    diagnostics.extend(check_application_stateful_counter_runtime_policy(core));
    diagnostics.extend(check_application_repeated_scheduler_temporal_policy(core));
    diagnostics.extend(check_application_repeated_scheduler_retry_backoff_policy(
        core,
    ));
    diagnostics.extend(check_toolchain_agent_artifact_fingerprint_reads(core));
    diagnostics.extend(check_tool_secret_output_disclosure(core));
    diagnostics.extend(check_unknown_field_references(core));
    diagnostics.extend(check_failure_handling(core));
    diagnostics.extend(check_failure_trace_coverage(core));
    diagnostics.extend(check_recursive_function_termination(core));
    diagnostics.extend(check_semantic_node_provenance(core));
    diagnostics.extend(check_guarantee_attachment(core));
    diagnostics.extend(check_trace_attachment(core));
    diagnostics.extend(check_rule_attachment(core));
    diagnostics.extend(check_effect_attachment(core));
    diagnostics.extend(check_imported_action_effect_grants(core));
    diagnostics.extend(check_secret_attachment(core));
    diagnostics.extend(check_tool_trace_coverage(core));
    diagnostics.extend(check_tool_approval_mentions(core));
    diagnostics.extend(check_tool_permission_mentions(core));
    diagnostics.extend(check_tool_provider_call_audit_evidence(core));
    diagnostics.extend(check_tool_provider_call_failure_policy(core));
    diagnostics.extend(check_tool_provider_failure_recovery_policy(core));
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
    diagnostics.extend(check_external_binding_trace_coverage(core));
    diagnostics.extend(check_external_binding_status_maps(core));
    diagnostics.extend(check_external_binding_pointer_ownership(core));
    diagnostics.extend(check_external_binding_owned_pointer_release(core));
    diagnostics.extend(check_external_binding_nullable_non_null(core));
    diagnostics.extend(check_external_binding_mutable_aliases(core));
    diagnostics.extend(check_external_binding_secret_leakage(core));
    diagnostics.extend(check_ui_form_action_targets(core));
    diagnostics.extend(check_ui_route_permissions(core));
    diagnostics.extend(check_ui_dashboard_permissions(core));
    diagnostics.extend(check_ui_form_accessibility(core));
    diagnostics.extend(check_ui_destructive_action_confirmations(core));
    diagnostics.extend(check_ui_workflow_step_order(core));
    for action in core.graph.nodes.iter().filter(|node| node.kind == "Action") {
        if !has_outgoing_edge(&core.graph, "records_trace", &action.id) {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-TRACE-001",
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

fn check_core_schema_version(core: &AilCore) -> Vec<AilDiagnostic> {
    let Some(schema_version) = &core.package.schema_version else {
        return Vec::new();
    };
    if schema_version == "ail-core.schema.v0" {
        return Vec::new();
    }
    vec![
        AilDiagnostic::error(
            "AIL-SCHEMA-003",
            format!("unknown AIL-Core schema version '{schema_version}'"),
        )
        .with_affected_graph_item(format!("package:{}", core.package.name))
        .with_repair_suggestion("Use ail-core.schema.v0 or migrate the package before checking."),
    ]
}

fn check_package_safety_level(core: &AilCore) -> Vec<AilDiagnostic> {
    let Some(safety_level) = &core.package.safety_level else {
        return Vec::new();
    };
    if matches!(
        safety_level.as_str(),
        "standard" | "low" | "medium" | "high" | "expert"
    ) {
        return Vec::new();
    }
    vec![
        AilDiagnostic::error(
            "AIL-SAFETY-001",
            format!("unknown AIL package safety level '{safety_level}'"),
        )
        .with_affected_graph_item(format!("package:{}", core.package.name))
        .with_repair_suggestion(
            "Use standard, low, medium, high, or expert as the package safety-level.",
        ),
    ]
}

fn check_package_target_support_statuses(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for (target, status) in &core.package.target_support {
        if is_known_target_support_status(status) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-BACKEND-002",
                format!("unknown AIL target-support status '{status}' for target {target}"),
            )
            .with_affected_graph_item(format!(
                "package:{} target-support:{target}",
                core.package.name
            ))
            .with_repair_suggestion(
                "Use supported, supported-with-host-imports, or planned-contract as the target-support status.",
            ),
        );
    }
    diagnostics
}

fn is_known_target_support_status(status: &str) -> bool {
    matches!(
        status,
        "supported" | "supported-with-host-imports" | "planned-contract"
    )
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
            | "Confirmation"
            | "CorpusFixture"
            | "Accessibility"
            | "Dashboard"
            | "Diagnostic"
            | "Effect"
            | "Event"
            | "ExecutionContext"
            | "ExternalBinding"
            | "Failure"
            | "Field"
            | "Filter"
            | "Form"
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
            | "Route"
            | "SchedulerTask"
            | "SchedulerTaskPriority"
            | "SchedulerTaskTiming"
            | "Secret"
            | "StatusMap"
            | "Step"
            | "SystemComponent"
            | "TerminationBound"
            | "TerminationMeasure"
            | "Thing"
            | "Tool"
            | "Trace"
            | "Type"
            | "User"
            | "Value"
            | "Variant"
            | "View"
            | "Workflow"
    )
}

fn is_known_core_edge_kind(kind: &str) -> bool {
    matches!(
        kind,
        "allocates_resource"
            | "authorizes_resource"
            | "blocks_before"
            | "borrows_resource"
            | "calls"
            | "contains"
            | "depends_on"
            | "emits"
            | "grants_permission"
            | "guards_resource"
            | "guarantees"
            | "handles_failure"
            | "has_accessibility"
            | "has_field"
            | "has_input"
            | "has_output"
            | "has_provenance"
            | "has_termination_bound"
            | "has_termination_measure"
            | "in_region"
            | "layouts_resource"
            | "lowers_to"
            | "maps_status"
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
            | "repeats"
            | "requires"
            | "requires_approval"
            | "requires_confirmation"
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
            | "filters"
            | "validates"
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
        format!(
            "capability-grants: {}",
            render_capability_grant_specs(&core.package.capability_grants)
        ),
        format!("conformance: {}", core.package.conformance),
    ];
    if let Some(prompt_pack) = &core.package.prompt_pack {
        lines.push(format!("prompt-pack: {prompt_pack}"));
    }
    if let Some(registry) = &core.package.registry {
        lines.push(format!("registry: {registry}"));
    }
    if !core.package.target_support.is_empty() {
        lines.push(format!(
            "target-support: {}",
            render_target_support_specs(&core.package.target_support)
        ));
    }
    if let Some(schema_version) = &core.package.schema_version {
        lines.push(format!("schema-version: {schema_version}"));
    }
    if let Some(safety_level) = &core.package.safety_level {
        lines.push(format!("safety-level: {safety_level}"));
    }
    lines.extend([
        format!("base_llm_endpoint: {}", core.package.base_llm_endpoint),
        String::new(),
        "nodes:".to_string(),
    ]);
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
    let mut edge_lines = core
        .graph
        .edges
        .iter()
        .map(|edge| {
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
            (edge.ordinal, edge.id.clone(), line)
        })
        .collect::<Vec<_>>();
    edge_lines.sort();
    for (_, _, line) in edge_lines {
        lines.push(line);
    }
    lines.join("\n")
}

fn render_import_specs(imports: &[AilImportSpec]) -> String {
    imports
        .iter()
        .map(|import| {
            let version = import
                .version
                .as_ref()
                .map(|version| format!("@{version}"))
                .unwrap_or_default();
            let registry_identity = import
                .registry_identity
                .as_ref()
                .map(|identity| format!(" registry {identity}"))
                .unwrap_or_default();
            let resolved_package = import
                .resolved_package
                .as_ref()
                .map(|package| format!(" resolved {package}"))
                .unwrap_or_default();
            format!(
                "{}{} as {}{}{}",
                import.path, version, import.alias, registry_identity, resolved_package
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_capability_grant_specs(grants: &[AilCapabilityGrant]) -> String {
    grants
        .iter()
        .map(|grant| {
            format!(
                "package={};capability={};effects={};approvals={}",
                grant.package,
                grant.capability,
                grant.effects.join("|"),
                grant.approvals.join("|")
            )
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

fn render_target_support_specs(target_support: &BTreeMap<String, String>) -> String {
    target_support
        .iter()
        .map(|(target, status)| format!("{target}={status}"))
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
    let mut capability_grants = Vec::new();
    let mut conformance = None;
    let mut prompt_pack = None;
    let mut registry = None;
    let mut target_support = BTreeMap::new();
    let mut schema_version = None;
    let mut safety_level = None;
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
                    "capability-grants" => {
                        capability_grants = if value.is_empty() {
                            Vec::new()
                        } else {
                            parse_capability_grant_specs(&value)?
                        };
                    }
                    "conformance" => conformance = Some(value),
                    "prompt-pack" => prompt_pack = Some(value),
                    "registry" => registry = Some(value),
                    "target-support" => target_support = parse_target_support_specs(&value)?,
                    "schema-version" => schema_version = Some(value),
                    "safety-level" => safety_level = Some(value),
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
            capability_grants,
            conformance: conformance
                .ok_or_else(|| "AIL-Core missing conformance metadata".to_string())?,
            prompt_pack,
            registry,
            target_support,
            schema_version,
            safety_level,
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
    let routes = sorted_node_names(core, "Route")
        .into_iter()
        .map(|route| render_flow_ui_surface(core, "Route", &route))
        .collect::<Vec<_>>()
        .join(",");
    let forms = sorted_node_names(core, "Form")
        .into_iter()
        .map(|form| render_flow_ui_surface(core, "Form", &form))
        .collect::<Vec<_>>()
        .join(",");
    let dashboards = sorted_node_names(core, "Dashboard")
        .into_iter()
        .map(|dashboard| render_flow_ui_surface(core, "Dashboard", &dashboard))
        .collect::<Vec<_>>()
        .join(",");
    let workflows = sorted_node_names(core, "Workflow")
        .into_iter()
        .map(|workflow| render_flow_ui_surface(core, "Workflow", &workflow))
        .collect::<Vec<_>>()
        .join(",");
    let accessibility = sorted_node_names(core, "Accessibility")
        .into_iter()
        .map(|item| {
            let core_label = flow_core_label(core, "Accessibility", &item);
            format!(
                "{{\"name\":{},\"coreLabel\":{}}}",
                json_string(&item),
                json_string(&core_label)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
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
        "{{\"kind\":\"AIL-Flow\",\"package\":{},\"coreHash\":{},\"application\":{},\"things\":[{}],\"views\":{},\"routes\":[{}],\"forms\":[{}],\"dashboards\":[{}],\"workflows\":[{}],\"accessibility\":[{}],\"actions\":[{}],\"tools\":[{}],\"compilerPasses\":[{}],\"systemComponents\":[{}]}}",
        json_string(&core.package.name),
        json_string(&core_hash),
        json_string(&application),
        things,
        views,
        routes,
        forms,
        dashboards,
        workflows,
        accessibility,
        actions,
        tools,
        compiler_passes,
        system_components
    )
}

fn render_flow_ui_surface(core: &AilCore, kind: &str, name: &str) -> String {
    let Some(node) = core.graph.find_node(kind, name) else {
        let core_label = flow_core_label(core, kind, name);
        return format!(
            "{{\"name\":{},\"coreLabel\":{},\"label\":\"\",\"reads\":[],\"requires\":[],\"fields\":[],\"calls\":[],\"validations\":[],\"filters\":[],\"steps\":[],\"traces\":[],\"accessibility\":[],\"edgeRefs\":[]}}",
            json_string(name),
            json_string(&core_label)
        );
    };
    let label = node
        .attributes
        .get("label")
        .map(String::as_str)
        .unwrap_or("");
    let path = node
        .attributes
        .get("path")
        .map(String::as_str)
        .unwrap_or("");
    format!(
        "{{\"name\":{},\"coreLabel\":{},\"label\":{},\"path\":{},\"reads\":{},\"requires\":{},\"fields\":{},\"calls\":{},\"validations\":{},\"filters\":{},\"steps\":{},\"traces\":{},\"accessibility\":{},\"edgeRefs\":{}}}",
        json_string(name),
        json_string(&core_node_label(node)),
        json_string(label),
        json_string(path),
        render_json_array(edge_target_names(core, &node.id, "reads")),
        render_json_array(edge_target_names(core, &node.id, "requires")),
        render_json_array(edge_target_names(core, &node.id, "has_field")),
        render_json_array(edge_target_names(core, &node.id, "calls")),
        render_json_array(edge_target_names(core, &node.id, "validates")),
        render_json_array(edge_target_names(core, &node.id, "filters")),
        render_json_array(edge_target_names(core, &node.id, "contains")),
        render_json_array(edge_target_names(core, &node.id, "records_trace")),
        render_json_array(edge_target_names(core, &node.id, "has_accessibility")),
        render_flow_edge_refs(
            core,
            node,
            &[
                "reads",
                "requires",
                "has_field",
                "calls",
                "validates",
                "filters",
                "contains",
                "records_trace",
                "requires_confirmation",
                "has_accessibility",
            ],
        )
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
            "{{\"name\":{},\"coreLabel\":{},\"label\":\"\",\"trigger\":\"\",\"requires\":[],\"reads\":[],\"writes\":[],\"calls\":[],\"guarantees\":[],\"traces\":[],\"edgeRefs\":[]}}",
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
        "{{\"name\":{},\"coreLabel\":{},\"label\":{},\"trigger\":{},\"requires\":{},\"reads\":{},\"writes\":{},\"calls\":{},\"guarantees\":{},\"traces\":{},\"edgeRefs\":{}}}",
        json_string(action),
        json_string(&core_node_label(action_node)),
        json_string(label),
        json_string(trigger),
        render_json_array(edge_target_names(core, &action_node.id, "requires")),
        render_json_array(edge_target_names(core, &action_node.id, "reads")),
        render_json_array(edge_target_names(core, &action_node.id, "writes")),
        render_json_array(edge_target_names(core, &action_node.id, "calls")),
        render_json_array(edge_target_names(core, &action_node.id, "guarantees")),
        render_json_array(edge_target_names(core, &action_node.id, "records_trace")),
        render_flow_edge_refs(
            core,
            action_node,
            &[
                "requires",
                "reads",
                "writes",
                "calls",
                "guarantees",
                "records_trace"
            ]
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
    for type_decl in document.types.values() {
        lines.push(format!("Type: {}.", type_decl.label));
        lines.push(String::new());
        if !type_decl.variants.is_empty() {
            lines.push(format!(
                "{} has variants:",
                type_base_name(&type_decl.label)
            ));
            lines.push(String::new());
            for variant in type_decl.variants.values() {
                lines.push(format!("- {}", render_variant_spec(variant)));
            }
            lines.push(String::new());
        }
    }
    for route in document.routes.values() {
        lines.push(format!("Route: {}.", route.label));
        lines.push(String::new());
        if !route.path.is_empty() {
            lines.push("The route path is:".to_string());
            lines.push(String::new());
            lines.push(format!("- {}", route.path));
            lines.push(String::new());
        }
        if !route.reads.is_empty() {
            lines.push("The route reads:".to_string());
            lines.push(String::new());
            for read in &route.reads {
                lines.push(format!("- {read}"));
            }
            lines.push(String::new());
        }
        if !route.permissions.is_empty() {
            lines.push("The route requires permission:".to_string());
            lines.push(String::new());
            for permission in &route.permissions {
                lines.push(format!("- {permission}"));
            }
            lines.push(String::new());
        }
        if !route.traces.is_empty() {
            lines.push("The route records trace:".to_string());
            lines.push(String::new());
            for trace in &route.traces {
                lines.push(format!("- {trace}"));
            }
            lines.push(String::new());
        }
    }
    for function in document.functions.values() {
        lines.push(format!("Function: {}.", function.label));
        lines.push(String::new());
        if !function.inputs.is_empty() {
            lines.push("The function needs:".to_string());
            lines.push(String::new());
            for input in function.inputs.values() {
                lines.push(format!("- {}: {}", input.name, input.type_name));
            }
            lines.push(String::new());
        }
        if !function.outputs.is_empty() {
            lines.push("The function produces:".to_string());
            lines.push(String::new());
            for output in function.outputs.values() {
                lines.push(format!("- {}: {}", output.name, output.type_name));
            }
            lines.push(String::new());
        }
        if !(function.branches.is_empty()
            && function.calls.is_empty()
            && function.termination_bounds.is_empty()
            && function.termination_measures.is_empty()
            && function.returns.is_empty()
            && function.traces.is_empty())
        {
            lines.push(format!("When {} runs:", function.label));
            lines.push(String::new());
            for branch in &function.branches {
                lines.push(format!("- if {branch}"));
            }
            for call in &function.calls {
                lines.push(format!("- otherwise the function calls {}", call.text));
            }
            for bound in &function.termination_bounds {
                lines.push(format!("- the function has {bound}"));
            }
            for measure in &function.termination_measures {
                lines.push(format!("- the function has {measure}"));
            }
            for return_value in &function.returns {
                lines.push(format!("- the function returns {return_value}"));
            }
            for trace in &function.traces {
                lines.push(format!(
                    "- the function records a trace event named {trace}"
                ));
            }
            lines.push(String::new());
        }
    }
    for binding in document.external_bindings.values() {
        lines.push(format!("C library: {}.", binding.library));
        lines.push(String::new());
        lines.push(format!("The library imports function {}.", binding.symbol));
        lines.push(String::new());
        if !binding.inputs.is_empty() {
            lines.push(format!("{} needs:", binding.symbol));
            lines.push(String::new());
            for input in binding.inputs.values() {
                lines.push(format!(
                    "- {}: {}",
                    input.name,
                    render_external_binding_value_type(input)
                ));
            }
            lines.push(String::new());
        }
        if !binding.outputs.is_empty() {
            lines.push(format!("{} produces:", binding.symbol));
            lines.push(String::new());
            for output in binding.outputs.values() {
                lines.push(format!(
                    "- {}: {}",
                    output.name,
                    render_external_binding_value_type(output)
                ));
            }
            lines.push(String::new());
        }
        if !binding.status_maps.is_empty() {
            lines.push(format!("{} maps errno or status codes:", binding.symbol));
            lines.push(String::new());
            for status_map in &binding.status_maps {
                lines.push(format!(
                    "- {} maps to {}",
                    status_map.code, status_map.target
                ));
            }
            lines.push(String::new());
        }
        if !binding.capabilities.is_empty() {
            lines.push(format!("{} requires capability:", binding.symbol));
            lines.push(String::new());
            for capability in &binding.capabilities {
                lines.push(format!("- {capability}"));
            }
            lines.push(String::new());
        }
        for trace in &binding.traces {
            lines.push(format!(
                "{} records trace event named {trace}",
                binding.symbol
            ));
            lines.push(String::new());
        }
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
        for call in &action.calls {
            lines.push(format!("- the system calls {call}"));
        }
        for repeated_call in &action.repeated_calls {
            lines.push(format!(
                "- the system repeats {} {} times",
                repeated_call.target, repeated_call.count
            ));
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
    run_ail_action_inner(document, action_name, runtime_state, 0)
}

fn run_ail_action_inner(
    document: &AilDocument,
    action_name: &str,
    runtime_state: BTreeMap<String, String>,
    call_depth: usize,
) -> Result<AilRunResult, String> {
    if call_depth > 64 {
        return Err(format!(
            "AIL action call depth exceeded while calling {action_name}"
        ));
    }
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

    for call in &action.calls {
        let target =
            action_call_target_name(document, call).unwrap_or_else(|| action_name_from_label(call));
        trace.push(format!("call action {target}"));
        let mut called = run_ail_action_inner(document, &target, final_state, call_depth + 1)?;
        trace.append(&mut called.trace);
        if called.status != "succeeded" {
            return Ok(AilRunResult {
                status: called.status,
                failure: called.failure,
                final_state: called.final_state,
                trace,
            });
        }
        final_state = called.final_state;
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
    ensure_native_target_supported(&program.package_name, &program.target_support, target)?;
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
    let action = expand_native_action_calls(action, &program.actions)?;
    emit_linux_x86_64_elf_for_action(&action, &program.failures)
}

fn ensure_native_target_supported(
    package_name: &str,
    target_support: &BTreeMap<String, String>,
    target: &str,
) -> Result<(), String> {
    if target_support.is_empty() {
        return Ok(());
    }
    for target_name in target_support_lookup_names(target) {
        if let Some(status) = target_support.get(*target_name) {
            if status == "supported" {
                return Ok(());
            }
            return Err(format!(
                "AIL-BACKEND-001 package {} target-support marks {target_name} as {status}; native target {target} requires supported",
                package_name
            ));
        }
    }
    Err(format!(
        "AIL-BACKEND-001 package {} target-support does not declare native target {target}",
        package_name
    ))
}

fn target_support_lookup_names(target: &str) -> &'static [&'static str] {
    match target {
        "linux-x86_64-elf" => &["linux-x86_64-elf", "x86_64-unknown-linux-syscall-elf"],
        _ => &[],
    }
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
    ensure_native_target_supported(&program.package_name, &program.target_support, target)?;
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
    let action = expand_native_action_calls(action, &program.actions)?;
    emit_linux_x86_64_elf_for_action(&action, &program.failures)
}

struct NativeFailureBranch {
    label: String,
    trace_lines: Vec<String>,
}

fn expand_native_action_calls(
    action: &AilBytecodeAction,
    actions: &BTreeMap<String, AilBytecodeAction>,
) -> Result<AilBytecodeAction, String> {
    let mut next_call_id = 0usize;
    let instructions =
        expand_native_action_call_stream(action, actions, false, "", &mut next_call_id, 0)?;
    Ok(AilBytecodeAction {
        name: action.name.clone(),
        instructions: annotate_native_integer_state_effects(instructions)?,
    })
}

fn annotate_native_integer_state_effects(
    instructions: Vec<AilBytecodeInstruction>,
) -> Result<Vec<AilBytecodeInstruction>, String> {
    let mut integer_deltas = BTreeMap::<String, i64>::new();
    let mut annotated = Vec::with_capacity(instructions.len());
    for mut instruction in instructions {
        match instruction.opcode.as_str() {
            "ADD_INT_FIELD" => {
                let key = instruction
                    .operands
                    .get("key")
                    .ok_or_else(|| "native ADD_INT_FIELD is missing key".to_string())?
                    .clone();
                let delta = instruction
                    .operands
                    .get("delta")
                    .ok_or_else(|| {
                        format!("native ADD_INT_FIELD for field {key} is missing delta")
                    })?
                    .parse::<i64>()
                    .map_err(|_| {
                        format!("native ADD_INT_FIELD for field {key} has non-integer delta")
                    })?;
                let effective_delta = integer_deltas.entry(key).or_insert(0);
                *effective_delta += delta;
                instruction.operands.insert(
                    "native_effective_delta".to_string(),
                    effective_delta.to_string(),
                );
            }
            "LABEL" | "BRANCH_FIELD_EQUALS" | "JUMP" => integer_deltas.clear(),
            _ => {}
        }
        annotated.push(instruction);
    }
    Ok(annotated)
}

fn expand_native_action_call_stream(
    action: &AilBytecodeAction,
    actions: &BTreeMap<String, AilBytecodeAction>,
    inline: bool,
    label_prefix: &str,
    next_call_id: &mut usize,
    call_depth: usize,
) -> Result<Vec<AilBytecodeInstruction>, String> {
    if call_depth > 64 {
        return Err(format!(
            "native linux-x86_64-elf action call depth exceeded while inlining {}",
            action.name
        ));
    }
    let mut instructions = Vec::new();
    for (index, instruction) in action.instructions.iter().enumerate() {
        if instruction.opcode == "CALL_ACTION" {
            let target = instruction.operands.get("target").ok_or_else(|| {
                format!(
                    "native linux-x86_64-elf CALL_ACTION in action {} is missing target",
                    action.name
                )
            })?;
            let callee = actions.get(target).ok_or_else(|| {
                format!(
                    "native linux-x86_64-elf CALL_ACTION in action {} targets unknown action {}",
                    action.name, target
                )
            })?;
            instructions.push(AilBytecodeInstruction::new(
                "NATIVE_TRACE_LINE",
                &[("text", format!("call action {target}"))],
            ));
            let call_id = *next_call_id;
            *next_call_id += 1;
            let callee_label_prefix = format!("__call{call_id}_{target}_");
            instructions.extend(expand_native_action_call_stream(
                callee,
                actions,
                true,
                &callee_label_prefix,
                next_call_id,
                call_depth + 1,
            )?);
            continue;
        }
        if instruction.opcode == "REPEAT_ACTION" {
            let target = instruction.operands.get("target").ok_or_else(|| {
                format!(
                    "native linux-x86_64-elf REPEAT_ACTION in action {} is missing target",
                    action.name
                )
            })?;
            let count = instruction
                .operands
                .get("count")
                .ok_or_else(|| {
                    format!(
                        "native linux-x86_64-elf REPEAT_ACTION in action {} is missing count",
                        action.name
                    )
                })?
                .parse::<usize>()
                .map_err(|_| {
                    format!(
                        "native linux-x86_64-elf REPEAT_ACTION in action {} has non-integer count",
                        action.name
                    )
                })?;
            if count == 0 {
                return Err(format!(
                    "native linux-x86_64-elf REPEAT_ACTION in action {} has zero count",
                    action.name
                ));
            }
            let callee = actions.get(target).ok_or_else(|| {
                format!(
                    "native linux-x86_64-elf REPEAT_ACTION in action {} targets unknown action {}",
                    action.name, target
                )
            })?;
            instructions.push(AilBytecodeInstruction::new(
                "NATIVE_TRACE_LINE",
                &[("text", format!("repeat action {target} {count} times"))],
            ));
            for iteration in 1..=count {
                instructions.push(AilBytecodeInstruction::new(
                    "NATIVE_TRACE_LINE",
                    &[("text", format!("repeat {target} iteration {iteration}"))],
                ));
                let call_id = *next_call_id;
                *next_call_id += 1;
                let callee_label_prefix = format!("__repeat{call_id}_{target}_");
                instructions.extend(expand_native_action_call_stream(
                    callee,
                    actions,
                    true,
                    &callee_label_prefix,
                    next_call_id,
                    call_depth + 1,
                )?);
            }
            continue;
        }
        if inline && instruction.opcode == "RETURN_SUCCESS" {
            if index + 1 == action.instructions.len() {
                continue;
            }
            return Err(format!(
                "unsupported native linux-x86_64-elf early RETURN_SUCCESS in called action {}",
                action.name
            ));
        }
        instructions.push(native_prefixed_label_instruction(instruction, label_prefix));
    }
    Ok(instructions)
}

fn native_prefixed_label_instruction(
    instruction: &AilBytecodeInstruction,
    label_prefix: &str,
) -> AilBytecodeInstruction {
    if label_prefix.is_empty() {
        return instruction.clone();
    }
    let mut instruction = instruction.clone();
    match instruction.opcode.as_str() {
        "LABEL" => {
            if let Some(name) = instruction.operands.get_mut("name") {
                *name = format!("{label_prefix}{name}");
            }
        }
        "BRANCH_FIELD_EQUALS" | "JUMP" => {
            if let Some(label) = instruction.operands.get_mut("label") {
                *label = format!("{label_prefix}{label}");
            }
        }
        _ => {}
    }
    instruction
}

fn emit_linux_x86_64_elf_for_action(
    action: &AilBytecodeAction,
    failures: &BTreeMap<String, AilBytecodeFailure>,
) -> Result<Vec<u8>, String> {
    let mut failure_branches = Vec::new();
    let mut data_labels = Vec::new();
    let mut next_data_label = 0usize;
    let (bytecode_labels, label_diagnostics) = ail_bytecode_action_labels(action);
    if !label_diagnostics.is_empty() {
        return Err(label_diagnostics.join("\n"));
    }
    let label_by_instruction = bytecode_labels
        .iter()
        .map(|(name, index)| (*index, native_bytecode_label(name)))
        .collect::<BTreeMap<_, _>>();

    let mut code = X64Code::default();
    code.emit(&[
        0x48, 0x8b, 0x1c, 0x24, // mov rbx, [rsp]
        0x4c, 0x8d, 0x64, 0x24, 0x10, // lea r12, [rsp+16]
        0x49, 0x89, 0xdd, // mov r13, rbx
        0x49, 0xff, 0xcd, // dec r13
    ]);

    for (instruction_index, instruction) in action.instructions.iter().enumerate() {
        if let Some(label) = label_by_instruction.get(&instruction_index) {
            code.label(label.clone())?;
        }
        match instruction.opcode.as_str() {
            "ACTION_BEGIN" => {
                if let Some(action) = instruction.operands.get("action") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("action {action} started\n"),
                    );
                }
            }
            "FUNCTION_BEGIN" => {
                if let Some(label) = instruction.operands.get("label") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("function {label} started\n"),
                    );
                }
            }
            "FUNCTION_INPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("function input {name}:{type_name}\n"),
                    );
                }
            }
            "FUNCTION_OUTPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("function output {name}:{type_name}\n"),
                    );
                }
            }
            "FUNCTION_BRANCH" => {
                if let Some(condition) = instruction.operands.get("condition") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("function branch {condition}\n"),
                    );
                }
            }
            "FUNCTION_CALL" => {
                if let Some(target) = instruction.operands.get("target") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("function call {target}\n"),
                    );
                }
            }
            "FUNCTION_RETURN" => {
                if let Some(value) = instruction.operands.get("value") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("function return {value}\n"),
                    );
                }
            }
            "NATIVE_TRACE_LINE" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("{text}\n"),
                    );
                }
            }
            "LABEL" => {}
            "BRANCH_FIELD_EQUALS" => {
                if let (Some(key), Some(value), Some(label)) = (
                    instruction.operands.get("key"),
                    instruction.operands.get("value"),
                    instruction.operands.get("label"),
                ) {
                    let (target_index, target_label) = native_target_label(
                        &bytecode_labels,
                        &label_by_instruction,
                        label,
                        &action.name,
                    )?;
                    if target_index <= instruction_index {
                        return Err(format!(
                            "unsupported native linux-x86_64-elf backward BRANCH_FIELD_EQUALS from instruction {instruction_index} to label {label} in action '{}'",
                            action.name
                        ));
                    }
                    let exact_label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "branch_exact",
                        format!("{key}={value}"),
                    );
                    let taken_label = format!("branch_taken_{instruction_index}");
                    let done_label = format!("branch_done_{instruction_index}");
                    code.emit_lea_rsi_label(&exact_label);
                    code.emit_mov_edx_imm32(format!("{key}={value}").len() as u32);
                    code.emit_call_label("has_exact");
                    code.emit(&[0x85, 0xc0]); // test eax, eax
                    code.emit_jcc_label(&[0x0f, 0x85], &taken_label); // jnz taken
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("branch {label} skipped\n"),
                    );
                    code.emit_jmp_label(&done_label);
                    code.label(taken_label)?;
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("branch {label} taken\n"),
                    );
                    code.emit_jmp_label(&target_label);
                    code.label(done_label)?;
                }
            }
            "JUMP" => {
                if let Some(label) = instruction.operands.get("label") {
                    let (target_index, target_label) = native_target_label(
                        &bytecode_labels,
                        &label_by_instruction,
                        label,
                        &action.name,
                    )?;
                    if target_index <= instruction_index {
                        return Err(format!(
                            "unsupported native linux-x86_64-elf backward JUMP from instruction {instruction_index} to label {label} in action '{}'",
                            action.name
                        ));
                    }
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("jump {label}\n"),
                    );
                    code.emit_jmp_label(&target_label);
                }
            }
            "ADD_INT_FIELD" => {
                if let (Some(key), Some(delta)) = (
                    instruction.operands.get("key"),
                    instruction.operands.get("delta"),
                ) {
                    let delta = delta.parse::<i64>().map_err(|_| {
                        format!(
                            "unsupported native linux-x86_64-elf ADD_INT_FIELD delta '{delta}' in action '{}'",
                            action.name
                        )
                    })?;
                    let effective_delta = instruction
                        .operands
                        .get("native_effective_delta")
                        .map(|value| {
                            value.parse::<i64>().map_err(|_| {
                                format!(
                                    "unsupported native linux-x86_64-elf ADD_INT_FIELD effective delta in action '{}'",
                                    action.name
                                )
                            })
                        })
                        .transpose()?
                        .unwrap_or(delta);
                    let key_prefix = format!("{key}=");
                    let key_label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "add_int_key",
                        &key_prefix,
                    );
                    let stdout_prefix_label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "add_int_stdout_prefix",
                        &key_prefix,
                    );
                    let trace_prefix = format!("add {key} by {delta} -> ");
                    let trace_prefix_label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "add_int_trace_prefix",
                        &trace_prefix,
                    );
                    let newline_label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "newline",
                        b"\n",
                    );
                    let fail_label = format!("add_int_fail_{instruction_index}");
                    let done_label = format!("add_int_done_{instruction_index}");
                    code.emit_lea_rsi_label(&key_label);
                    code.emit_mov_edx_imm32(key_prefix.len() as u32);
                    code.emit_call_label("find_prefix");
                    code.emit_test_rax_rax();
                    code.emit_jcc_label(&[0x0f, 0x84], &fail_label); // jz fail
                    code.emit_mov_rdi_rax();
                    code.emit_call_label("parse_i64");
                    code.emit_test_edx_edx();
                    code.emit_jcc_label(&[0x0f, 0x84], &fail_label); // jz fail
                    code.emit_mov_r14_rax();
                    code.emit_mov_r15_imm64(effective_delta as u64);
                    code.emit_add_r14_r15();
                    code.emit_write_label(1, &stdout_prefix_label, key_prefix.len() as u32);
                    code.emit_mov_rdi_r14();
                    code.emit_mov_esi_imm32(1);
                    code.emit_call_label("write_i64");
                    code.emit_write_label(1, &newline_label, 1);
                    code.emit_write_label(2, &trace_prefix_label, trace_prefix.len() as u32);
                    code.emit_mov_rdi_r14();
                    code.emit_mov_esi_imm32(2);
                    code.emit_call_label("write_i64");
                    code.emit_write_label(2, &newline_label, 1);
                    code.emit_jmp_label(&done_label);
                    code.label(fail_label)?;
                    code.emit_exit(1);
                    code.label(done_label)?;
                }
            }
            "REQUIRE_EXISTS" => {
                if let Some(key) = instruction.operands.get("key") {
                    let fail_label = format!("fail_requirement_{}", failure_branches.len());
                    let prefix = format!("{key}=");
                    let prefix_label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "exists_prefix",
                        &prefix,
                    );
                    failure_branches.push(NativeFailureBranch {
                        label: fail_label,
                        trace_lines: native_failure_trace_lines(
                            &[],
                            instruction.operands.get("failure"),
                            failures,
                        ),
                    });
                    code.emit_lea_rsi_label(&prefix_label);
                    code.emit_mov_edx_imm32(prefix.len() as u32);
                    code.emit_call_label("has_prefix");
                    code.emit(&[0x85, 0xc0]); // test eax, eax
                    code.emit_jcc_label(&[0x0f, 0x84], &failure_branches.last().unwrap().label);
                    // jz fail
                }
                if let Some(rule) = instruction.operands.get("rule") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("rule passed: {rule}\n"),
                    );
                }
            }
            "REQUIRE_FIELD_NOT_EQUALS" => {
                if let (Some(key), Some(value)) = (
                    instruction.operands.get("key"),
                    instruction.operands.get("value"),
                ) {
                    let fail_label = format!("fail_requirement_{}", failure_branches.len());
                    let exact = format!("{key}={value}");
                    let exact_label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "forbidden_arg",
                        &exact,
                    );
                    failure_branches.push(NativeFailureBranch {
                        label: fail_label,
                        trace_lines: native_failure_trace_lines(
                            &[],
                            instruction.operands.get("failure"),
                            failures,
                        ),
                    });
                    code.emit_lea_rsi_label(&exact_label);
                    code.emit_mov_edx_imm32(exact.len() as u32);
                    code.emit_call_label("has_exact");
                    code.emit(&[0x85, 0xc0]); // test eax, eax
                    code.emit_jcc_label(&[0x0f, 0x85], &failure_branches.last().unwrap().label);
                    // jnz fail
                }
                if let Some(rule) = instruction.operands.get("rule") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("rule passed: {rule}\n"),
                    );
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
                        failure_branches.push(NativeFailureBranch {
                            label: fail_label,
                            trace_lines: native_failure_trace_lines(
                                &[],
                                instruction.operands.get("failure"),
                                failures,
                            ),
                        });
                        let matched_label = format!("field_in_matched_{instruction_index}");
                        for exact in allowed_args {
                            let label = push_native_data_label(
                                &mut data_labels,
                                &mut next_data_label,
                                "field_in_arg",
                                &exact,
                            );
                            code.emit_lea_rsi_label(&label);
                            code.emit_mov_edx_imm32(exact.len() as u32);
                            code.emit_call_label("has_exact");
                            code.emit(&[0x85, 0xc0]); // test eax, eax
                            code.emit_jcc_label(&[0x0f, 0x85], &matched_label); // jnz matched
                        }
                        code.emit_jmp_label(&failure_branches.last().unwrap().label);
                        code.label(matched_label)?;
                    }
                }
                if let Some(rule) = instruction.operands.get("rule") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("rule passed: {rule}\n"),
                    );
                }
            }
            "REQUIRE_FIELD_AFTER" => {
                if let (Some(source), Some(key)) = (
                    instruction.operands.get("source"),
                    instruction.operands.get("key"),
                ) {
                    let fail_label = format!("fail_requirement_{}", failure_branches.len());
                    let source_prefix = format!("{source}=");
                    let key_prefix = format!("{key}=");
                    let source_label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "field_after_source",
                        &source_prefix,
                    );
                    let key_label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "field_after_key",
                        &key_prefix,
                    );
                    failure_branches.push(NativeFailureBranch {
                        label: fail_label,
                        trace_lines: native_failure_trace_lines(
                            &[],
                            instruction.operands.get("failure"),
                            failures,
                        ),
                    });
                    let fail_label = failure_branches.last().unwrap().label.clone();
                    code.emit_lea_rsi_label(&source_label);
                    code.emit_mov_edx_imm32(source_prefix.len() as u32);
                    code.emit_call_label("find_prefix");
                    code.emit_test_rax_rax();
                    code.emit_jcc_label(&[0x0f, 0x84], &fail_label); // jz fail
                    code.emit_mov_r14_rax();
                    code.emit_lea_rsi_label(&key_label);
                    code.emit_mov_edx_imm32(key_prefix.len() as u32);
                    code.emit_call_label("find_prefix");
                    code.emit_test_rax_rax();
                    code.emit_jcc_label(&[0x0f, 0x84], &fail_label); // jz fail
                    code.emit_mov_r15_rax();
                    code.emit_mov_rdi_r14();
                    code.emit_mov_rsi_r15();
                    code.emit_call_label("cstring_gt");
                    code.emit(&[0x85, 0xc0]); // test eax, eax
                    code.emit_jcc_label(&[0x0f, 0x84], &fail_label); // jz fail
                }
                if let Some(rule) = instruction.operands.get("rule") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("rule passed: {rule}\n"),
                    );
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
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("read {key}\n"),
                    );
                }
            }
            "READ_EFFECT" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("read {text}\n"),
                    );
                }
            }
            "SET_FIELD" => {
                if let (Some(key), Some(value)) = (
                    instruction.operands.get("key"),
                    instruction.operands.get("value"),
                ) {
                    let line = format!("{key}={value}\n");
                    let label = push_native_data_label(
                        &mut data_labels,
                        &mut next_data_label,
                        "state_write",
                        &line,
                    );
                    code.emit_write_label(1, &label, line.len() as u32);
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("write {key}={value}\n"),
                    );
                }
            }
            "COPY_FIELD" => {
                if let (Some(source), Some(key)) = (
                    instruction.operands.get("source"),
                    instruction.operands.get("key"),
                ) {
                    emit_native_copy_state_write(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        instruction_index,
                        &format!("{source}="),
                        &format!("{key}="),
                    )?;
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("write {key}\n"),
                    );
                }
            }
            "WRITE_FIELD" => {
                if let Some(key) = instruction.operands.get("key") {
                    emit_native_copy_state_write(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        instruction_index,
                        &format!("{key}.id="),
                        &format!("{key}.id="),
                    )?;
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("write {key}\n"),
                    );
                }
            }
            "EFFECT" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("effect {text}\n"),
                    );
                }
            }
            "ASSERT_GUARANTEE" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("guarantee checked: {text}\n"),
                    );
                }
            }
            "TOOL_BEGIN" => {
                if let Some(label) = instruction.operands.get("label") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool {label} started\n"),
                    );
                }
            }
            "TOOL_REQUIREMENT" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool requirement {text}\n"),
                    );
                }
            }
            "TOOL_INPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool input {name}:{type_name}\n"),
                    );
                }
            }
            "TOOL_OUTPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool output {name}:{type_name}\n"),
                    );
                }
            }
            "TOOL_READ" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool read {text}\n"),
                    );
                }
            }
            "TOOL_CALL" => {
                if let Some(target) = instruction.operands.get("target") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool call {target}\n"),
                    );
                }
            }
            "TOOL_WRITE" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool write {text}\n"),
                    );
                }
            }
            "TOOL_PERMISSION" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool permission {text}\n"),
                    );
                }
            }
            "TOOL_APPROVAL" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool approval {text}\n"),
                    );
                }
            }
            "TOOL_SECRET_PROTECTION" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("tool secret protection {text}\n"),
                    );
                }
            }
            "SYSTEM_BEGIN" => {
                if let Some(label) = instruction.operands.get("label") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system component {label} started\n"),
                    );
                }
            }
            "SYSTEM_RESOURCE" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system resource {name}:{type_name}\n"),
                    );
                }
            }
            "SYSTEM_OWNS" => {
                if let Some(resource) = instruction.operands.get("resource") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system owns {resource}\n"),
                    );
                }
            }
            "SYSTEM_BORROWS" => {
                if let Some(resource) = instruction.operands.get("resource") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system borrows {resource}\n"),
                    );
                }
            }
            "SYSTEM_MUTABLY_BORROWS" => {
                if let Some(resource) = instruction.operands.get("resource") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system mutably borrows {resource}\n"),
                    );
                }
            }
            "SYSTEM_REGION" => {
                if let (Some(resource), Some(region)) = (
                    instruction.operands.get("resource"),
                    instruction.operands.get("region"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system places {resource} in {region}\n"),
                    );
                }
            }
            "SYSTEM_LAYOUT" => {
                if let (Some(resource), Some(layout)) = (
                    instruction.operands.get("resource"),
                    instruction.operands.get("layout"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system layout {resource} {layout}\n"),
                    );
                }
            }
            "SYSTEM_ALLOCATION" => {
                if let (Some(resource), Some(placement)) = (
                    instruction.operands.get("resource"),
                    instruction.operands.get("placement"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system allocation {resource} {placement}\n"),
                    );
                }
            }
            "SYSTEM_LOCK_GUARD" => {
                if let (Some(resource), Some(lock)) = (
                    instruction.operands.get("resource"),
                    instruction.operands.get("lock"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system lock guard {resource} with {lock}\n"),
                    );
                }
            }
            "SYSTEM_CONTEXT" => {
                if let Some(name) = instruction.operands.get("name") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system context {name}\n"),
                    );
                }
            }
            "SYSTEM_INTERRUPT_PRIORITY" => {
                if let (Some(context), Some(priority)) = (
                    instruction.operands.get("context"),
                    instruction.operands.get("priority"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system interrupt priority {context} {priority}\n"),
                    );
                }
            }
            "SYSTEM_INTERRUPT_MASK" => {
                if let (Some(context), Some(mask)) = (
                    instruction.operands.get("context"),
                    instruction.operands.get("mask"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system interrupt mask {context} {mask}\n"),
                    );
                }
            }
            "SYSTEM_SCHEDULER_TASK" => {
                if let (Some(task), Some(context)) = (
                    instruction.operands.get("task"),
                    instruction.operands.get("context"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system scheduler task {task} in {context}\n"),
                    );
                }
            }
            "SYSTEM_TASK_PRIORITY" => {
                if let (Some(task), Some(priority)) = (
                    instruction.operands.get("task"),
                    instruction.operands.get("priority"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system task priority {task} {priority}\n"),
                    );
                }
            }
            "SYSTEM_TASK_TIMING" => {
                if let (Some(task), Some(deadline), Some(budget)) = (
                    instruction.operands.get("task"),
                    instruction.operands.get("deadline"),
                    instruction.operands.get("budget"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system task timing {task} deadline {deadline} budget {budget}\n"),
                    );
                }
            }
            "SYSTEM_CAPABILITY" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system capability {text}\n"),
                    );
                }
            }
            "SYSTEM_EFFECT" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("system effect {text}\n"),
                    );
                }
            }
            "PASS_BEGIN" => {
                if let Some(label) = instruction.operands.get("label") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("compiler pass {label} started\n"),
                    );
                }
            }
            "PASS_INPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("input {name}:{type_name}\n"),
                    );
                }
            }
            "PASS_OUTPUT" => {
                if let (Some(name), Some(type_name)) = (
                    instruction.operands.get("name"),
                    instruction.operands.get("type"),
                ) {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("output {name}:{type_name}\n"),
                    );
                }
            }
            "PASS_READ" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("pass read {text}\n"),
                    );
                }
            }
            "PASS_STEP" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("pass step {text}\n"),
                    );
                }
            }
            "PASS_WRITE" => {
                if let Some(text) = instruction.operands.get("text") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("pass write {text}\n"),
                    );
                }
            }
            "CORE_INFER_READ_PERMISSIONS" => {
                emit_native_trace_line(
                    &mut code,
                    &mut data_labels,
                    &mut next_data_label,
                    "core transform infer read permissions\n",
                );
            }
            "EMIT_TRACE" => {
                if let Some(event) = instruction.operands.get("event") {
                    emit_native_trace_line(
                        &mut code,
                        &mut data_labels,
                        &mut next_data_label,
                        &format!("trace {event}\n"),
                    );
                }
            }
            "RETURN_SUCCESS" => {
                code.emit_exit(0);
            }
            opcode => {
                return Err(format!(
                    "unsupported native linux-x86_64-elf opcode '{opcode}' in action '{}'",
                    action.name
                ));
            }
        }
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
    emit_parse_i64(&mut code)?;
    emit_write_i64(&mut code)?;
    for (label, bytes) in &data_labels {
        code.label(label.clone())?;
        code.emit(bytes);
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

fn native_bytecode_label(name: &str) -> String {
    format!("bytecode_label_{name}")
}

fn native_target_label(
    bytecode_labels: &BTreeMap<String, usize>,
    label_by_instruction: &BTreeMap<usize, String>,
    target: &str,
    action_name: &str,
) -> Result<(usize, String), String> {
    let target_index = bytecode_labels.get(target).ok_or_else(|| {
        format!("unknown native bytecode label '{target}' in action '{action_name}'")
    })?;
    let label = label_by_instruction
        .get(target_index)
        .cloned()
        .ok_or_else(|| {
            format!("unresolved native bytecode label '{target}' in action '{action_name}'")
        })?;
    Ok((*target_index, label))
}

fn push_native_data_label(
    data_labels: &mut Vec<(String, Vec<u8>)>,
    next_data_label: &mut usize,
    prefix: &str,
    bytes: impl AsRef<[u8]>,
) -> String {
    let label = format!("{prefix}_{}", *next_data_label);
    *next_data_label += 1;
    data_labels.push((label.clone(), bytes.as_ref().to_vec()));
    label
}

fn emit_native_trace_line(
    code: &mut X64Code,
    data_labels: &mut Vec<(String, Vec<u8>)>,
    next_data_label: &mut usize,
    line: &str,
) {
    let label = push_native_data_label(data_labels, next_data_label, "trace_write", line);
    code.emit_write_label(2, &label, line.len() as u32);
}

fn emit_native_copy_state_write(
    code: &mut X64Code,
    data_labels: &mut Vec<(String, Vec<u8>)>,
    next_data_label: &mut usize,
    instruction_index: usize,
    source_prefix: &str,
    destination_prefix: &str,
) -> Result<(), String> {
    let source_label = push_native_data_label(
        data_labels,
        next_data_label,
        "state_copy_source",
        source_prefix,
    );
    let destination_label = push_native_data_label(
        data_labels,
        next_data_label,
        "state_copy_destination",
        destination_prefix,
    );
    let newline_label =
        push_native_data_label(data_labels, next_data_label, "state_copy_newline", b"\n");
    let done_label = format!("state_copy_done_{instruction_index}");
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
    code.emit_write_label(1, &newline_label, 1);
    code.label(done_label)
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

fn emit_parse_i64(code: &mut X64Code) -> Result<(), String> {
    code.label("parse_i64")?;
    code.emit(&[
        0x48, 0x31, 0xc0, // xor rax, rax
        0x45, 0x31, 0xc0, // xor r8d, r8d
        0x45, 0x31, 0xd2, // xor r10d, r10d
        0x80, 0x3f, 0x2d, // cmp byte [rdi], '-'
    ]);
    code.emit_jcc_label(&[0x0f, 0x85], "parse_i64_loop"); // jne loop
    code.emit(&[
        0x41, 0xb8, 0x01, 0x00, 0x00, 0x00, // mov r8d, 1
        0x48, 0xff, 0xc7, // inc rdi
    ]);
    code.label("parse_i64_loop")?;
    code.emit(&[
        0x44, 0x8a, 0x0f, // mov r9b, [rdi]
        0x41, 0x80, 0xf9, 0x30, // cmp r9b, '0'
    ]);
    code.emit_jcc_label(&[0x0f, 0x82], "parse_i64_done"); // jb done
    code.emit(&[0x41, 0x80, 0xf9, 0x39]); // cmp r9b, '9'
    code.emit_jcc_label(&[0x0f, 0x87], "parse_i64_done"); // ja done
    code.emit(&[
        0x48, 0x6b, 0xc0, 0x0a, // imul rax, rax, 10
        0x49, 0x0f, 0xb6, 0xc9, // movzx rcx, r9b
        0x48, 0x83, 0xe9, 0x30, // sub rcx, '0'
        0x48, 0x01, 0xc8, // add rax, rcx
        0x49, 0xff, 0xc2, // inc r10
        0x48, 0xff, 0xc7, // inc rdi
    ]);
    code.emit_jmp_label("parse_i64_loop");
    code.label("parse_i64_done")?;
    code.emit(&[
        0x31, 0xd2, // xor edx, edx
        0x45, 0x85, 0xd2, // test r10d, r10d
    ]);
    code.emit_jcc_label(&[0x0f, 0x84], "parse_i64_ret"); // jz ret
    code.emit(&[0x45, 0x84, 0xc9]); // test r9b, r9b
    code.emit_jcc_label(&[0x0f, 0x85], "parse_i64_ret"); // jnz ret
    code.emit(&[
        0xba, 0x01, 0x00, 0x00, 0x00, // mov edx, 1
    ]);
    code.emit(&[0x45, 0x85, 0xc0]); // test r8d, r8d
    code.emit_jcc_label(&[0x0f, 0x84], "parse_i64_ret"); // jz ret
    code.emit(&[0x48, 0xf7, 0xd8]); // neg rax
    code.label("parse_i64_ret")?;
    code.emit(&[0xc3]); // ret
    Ok(())
}

fn emit_write_i64(code: &mut X64Code) -> Result<(), String> {
    code.label("write_i64")?;
    code.emit(&[
        0x48, 0x83, 0xec, 0x28, // sub rsp, 40
        0x48, 0x89, 0xf8, // mov rax, rdi
        0x41, 0x89, 0xf3, // mov r11d, esi
        0x45, 0x31, 0xc9, // xor r9d, r9d
        0x48, 0x85, 0xc0, // test rax, rax
    ]);
    code.emit_jcc_label(&[0x0f, 0x8d], "write_i64_positive"); // jge positive
    code.emit(&[
        0x48, 0xf7, 0xd8, // neg rax
        0x41, 0xb9, 0x01, 0x00, 0x00, 0x00, // mov r9d, 1
    ]);
    code.label("write_i64_positive")?;
    code.emit(&[
        0x4c, 0x8d, 0x54, 0x24, 0x28, // lea r10, [rsp+40]
        0x48, 0x85, 0xc0, // test rax, rax
    ]);
    code.emit_jcc_label(&[0x0f, 0x85], "write_i64_digit_loop"); // jne digit_loop
    code.emit(&[
        0x49, 0xff, 0xca, // dec r10
        0x41, 0xc6, 0x02, 0x30, // mov byte [r10], '0'
    ]);
    code.emit_jmp_label("write_i64_sign");
    code.label("write_i64_digit_loop")?;
    code.emit(&[
        0x31, 0xd2, // xor edx, edx
        0xb9, 0x0a, 0x00, 0x00, 0x00, // mov ecx, 10
        0x48, 0xf7, 0xf1, // div rcx
        0x80, 0xc2, 0x30, // add dl, '0'
        0x49, 0xff, 0xca, // dec r10
        0x41, 0x88, 0x12, // mov [r10], dl
        0x48, 0x85, 0xc0, // test rax, rax
    ]);
    code.emit_jcc_label(&[0x0f, 0x85], "write_i64_digit_loop"); // jne digit_loop
    code.label("write_i64_sign")?;
    code.emit(&[0x45, 0x85, 0xc9]); // test r9d, r9d
    code.emit_jcc_label(&[0x0f, 0x84], "write_i64_write"); // jz write
    code.emit(&[
        0x49, 0xff, 0xca, // dec r10
        0x41, 0xc6, 0x02, 0x2d, // mov byte [r10], '-'
    ]);
    code.label("write_i64_write")?;
    code.emit(&[
        0xb8, 0x01, 0x00, 0x00, 0x00, // mov eax, 1
        0x44, 0x89, 0xdf, // mov edi, r11d
        0x4c, 0x89, 0xd6, // mov rsi, r10
        0x48, 0x8d, 0x54, 0x24, 0x28, // lea rdx, [rsp+40]
        0x4c, 0x29, 0xd2, // sub rdx, r10
        0x0f, 0x05, // syscall
        0x48, 0x83, 0xc4, 0x28, // add rsp, 40
        0xc3, // ret
    ]);
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

    fn emit_test_edx_edx(&mut self) {
        self.emit(&[0x85, 0xd2]);
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

    fn emit_mov_rdi_rax(&mut self) {
        self.emit(&[0x48, 0x89, 0xc7]);
    }

    fn emit_mov_esi_imm32(&mut self, value: u32) {
        self.emit(&[0xbe]);
        self.emit(&value.to_le_bytes());
    }

    fn emit_mov_r15_imm64(&mut self, value: u64) {
        self.emit(&[0x49, 0xbf]);
        self.emit(&value.to_le_bytes());
    }

    fn emit_add_r14_r15(&mut self) {
        self.emit(&[0x4d, 0x01, 0xfe]);
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
        "Application" | "C interop" => {
            let mut actions = document
                .actions
                .iter()
                .map(|(name, action)| (name.clone(), compile_ail_action_bytecode(document, action)))
                .collect::<BTreeMap<_, _>>();
            for (name, function) in &document.functions {
                actions
                    .entry(name.clone())
                    .or_insert_with(|| compile_ail_function_bytecode(function));
            }
            actions
        }
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
        "UI" => compile_ail_ui_bytecode_actions(document),
        "System" => document
            .system_components
            .iter()
            .map(|(name, component)| (name.clone(), compile_ail_system_bytecode(component)))
            .collect(),
        profile => {
            return Err(format!(
                "ail-lower currently supports Application, C interop, AgentTool, Compiler, UI, and System packages, not {profile}"
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
        capability_grants: package.metadata.capability_grants.clone(),
        target_support: package.metadata.target_support.clone(),
        external_bindings_metadata_present: true,
        external_bindings: document.external_bindings.clone(),
        actions,
        failures,
    })
}

pub fn ail_document_from_core(core: &AilCore) -> AilDocument {
    let node_by_id = graph_node_by_id(core);
    let mut document = AilDocument {
        application: AilApplication::default(),
        things: BTreeMap::new(),
        tools: BTreeMap::new(),
        compiler_passes: BTreeMap::new(),
        system_components: BTreeMap::new(),
        functions: BTreeMap::new(),
        types: BTreeMap::new(),
        routes: BTreeMap::new(),
        forms: BTreeMap::new(),
        dashboards: BTreeMap::new(),
        workflows: BTreeMap::new(),
        external_bindings: BTreeMap::new(),
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

    for type_node in core.graph.nodes.iter().filter(|node| node.kind == "Type") {
        let mut type_decl = AilType {
            name: type_node.name.clone(),
            label: type_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| type_node.name.clone()),
            provenance: node_provenance(core, &type_node.id).unwrap_or_default(),
            ..AilType::default()
        };
        for variant_node in outgoing_nodes(core, &node_by_id, type_node, "contains")
            .into_iter()
            .filter(|node| node.kind == "Variant")
        {
            let variant_name = local_core_name(&variant_node.name, &type_decl.name);
            let mut variant = AilVariant {
                name: variant_name.clone(),
                label: variant_node
                    .attributes
                    .get("label")
                    .cloned()
                    .unwrap_or_else(|| variant_name.clone()),
                provenance: node_provenance(core, &variant_node.id).unwrap_or_default(),
                ..AilVariant::default()
            };
            for field_node in outgoing_nodes(core, &node_by_id, &variant_node, "has_field")
                .into_iter()
                .filter(|node| node.kind == "Field")
            {
                let field_name = local_core_name(&field_node.name, &variant_node.name);
                variant.fields.insert(
                    field_name.clone(),
                    AilVariantField {
                        name: field_name,
                        type_name: field_node.type_name.clone().unwrap_or_default(),
                        provenance: node_provenance(core, &field_node.id).unwrap_or_default(),
                    },
                );
            }
            type_decl.variants.insert(variant.name.clone(), variant);
        }
        document.types.insert(type_decl.name.clone(), type_decl);
    }

    for route_node in core.graph.nodes.iter().filter(|node| node.kind == "Route") {
        let route = AilRoute {
            name: route_node.name.clone(),
            label: route_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| route_node.name.clone()),
            path: route_node
                .attributes
                .get("path")
                .cloned()
                .unwrap_or_default(),
            reads: outgoing_nodes(core, &node_by_id, route_node, "reads")
                .into_iter()
                .filter(|node| node.kind == "Value")
                .map(|node| local_core_name(&node.name, &route_node.name))
                .collect(),
            permissions: outgoing_nodes(core, &node_by_id, route_node, "requires")
                .into_iter()
                .filter(|node| node.kind == "Permission")
                .map(|node| node.name)
                .collect(),
            traces: outgoing_nodes(core, &node_by_id, route_node, "records_trace")
                .into_iter()
                .filter(|node| node.kind == "Trace")
                .map(|node| node.name)
                .collect(),
            provenance: node_provenance(core, &route_node.id).unwrap_or_default(),
        };
        document.routes.insert(route.name.clone(), route);
    }

    for form_node in core.graph.nodes.iter().filter(|node| node.kind == "Form") {
        let mut form = AilForm {
            name: form_node.name.clone(),
            label: form_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| form_node.name.clone()),
            action: outgoing_nodes(core, &node_by_id, form_node, "calls")
                .into_iter()
                .find(|node| node.kind == "Action")
                .map(|node| node.name),
            provenance: node_provenance(core, &form_node.id).unwrap_or_default(),
            ..AilForm::default()
        };
        for field_node in outgoing_nodes(core, &node_by_id, form_node, "has_field")
            .into_iter()
            .filter(|node| node.kind == "Field")
        {
            let field_name = local_core_name(&field_node.name, &form.name);
            form.fields.insert(
                field_name.clone(),
                AilFormField {
                    name: field_name,
                    type_name: field_node.type_name.clone().unwrap_or_default(),
                    provenance: node_provenance(core, &field_node.id).unwrap_or_default(),
                },
            );
        }
        form.validations = outgoing_nodes(core, &node_by_id, form_node, "validates")
            .into_iter()
            .filter(|node| node.kind == "Rule")
            .map(|node| node.name)
            .collect();
        form.failure_traces = outgoing_nodes(core, &node_by_id, form_node, "records_trace")
            .into_iter()
            .filter(|node| node.kind == "Trace")
            .map(|node| node.name)
            .collect();
        form.confirmations = outgoing_nodes(core, &node_by_id, form_node, "requires_confirmation")
            .into_iter()
            .filter(|node| node.kind == "Confirmation")
            .map(|node| node.name)
            .collect();
        form.accessibility = outgoing_nodes(core, &node_by_id, form_node, "has_accessibility")
            .into_iter()
            .filter(|node| node.kind == "Accessibility")
            .map(|node| node.name)
            .collect();
        document.forms.insert(form.name.clone(), form);
    }

    for dashboard_node in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Dashboard")
    {
        let dashboard = AilDashboard {
            name: dashboard_node.name.clone(),
            label: dashboard_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| dashboard_node.name.clone()),
            reads: outgoing_nodes(core, &node_by_id, dashboard_node, "reads")
                .into_iter()
                .filter(|node| node.kind == "Value")
                .map(|node| local_core_name(&node.name, &dashboard_node.name))
                .collect(),
            permissions: outgoing_nodes(core, &node_by_id, dashboard_node, "requires")
                .into_iter()
                .filter(|node| node.kind == "Permission")
                .map(|node| node.name)
                .collect(),
            filters: outgoing_nodes(core, &node_by_id, dashboard_node, "filters")
                .into_iter()
                .filter(|node| node.kind == "Filter")
                .map(|node| node.name)
                .collect(),
            traces: outgoing_nodes(core, &node_by_id, dashboard_node, "records_trace")
                .into_iter()
                .filter(|node| node.kind == "Trace")
                .map(|node| node.name)
                .collect(),
            provenance: node_provenance(core, &dashboard_node.id).unwrap_or_default(),
        };
        document
            .dashboards
            .insert(dashboard.name.clone(), dashboard);
    }

    for workflow_node in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Workflow")
    {
        let steps = outgoing_nodes(core, &node_by_id, workflow_node, "contains")
            .into_iter()
            .filter(|node| node.kind == "Step")
            .map(|node| {
                node.attributes
                    .get("label")
                    .cloned()
                    .unwrap_or_else(|| local_core_name(&node.name, &workflow_node.name))
            })
            .collect::<Vec<_>>();
        let blocks = core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "blocks_before")
            .filter_map(|edge| {
                let blocked = node_by_id.get(&edge.source)?;
                let prerequisite = node_by_id.get(&edge.target)?;
                if blocked.kind != "Step"
                    || prerequisite.kind != "Step"
                    || !blocked
                        .name
                        .starts_with(&format!("{}.", workflow_node.name))
                    || !prerequisite
                        .name
                        .starts_with(&format!("{}.", workflow_node.name))
                {
                    return None;
                }
                Some(AilWorkflowBlock {
                    blocked_step: blocked
                        .attributes
                        .get("label")
                        .cloned()
                        .unwrap_or_else(|| local_core_name(&blocked.name, &workflow_node.name)),
                    prerequisite_step: prerequisite
                        .attributes
                        .get("label")
                        .cloned()
                        .unwrap_or_else(|| {
                            local_core_name(&prerequisite.name, &workflow_node.name)
                        }),
                    provenance: edge
                        .attributes
                        .get("provenance")
                        .cloned()
                        .unwrap_or_default(),
                })
            })
            .collect();
        let workflow = AilWorkflow {
            name: workflow_node.name.clone(),
            label: workflow_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| workflow_node.name.clone()),
            steps,
            blocks,
            traces: outgoing_nodes(core, &node_by_id, workflow_node, "records_trace")
                .into_iter()
                .filter(|node| node.kind == "Trace")
                .map(|node| node.name)
                .collect(),
            provenance: node_provenance(core, &workflow_node.id).unwrap_or_default(),
        };
        document.workflows.insert(workflow.name.clone(), workflow);
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

    for function_node in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Function")
    {
        let mut function = AilFunction {
            name: function_node.name.clone(),
            label: function_node
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| function_node.name.clone()),
            provenance: node_provenance(core, &function_node.id).unwrap_or_default(),
            ..AilFunction::default()
        };
        for input_node in outgoing_nodes(core, &node_by_id, function_node, "has_input")
            .into_iter()
            .filter(|node| node.kind == "Input")
        {
            let input_name = local_core_name(&input_node.name, &function.name);
            function.inputs.insert(
                input_name.clone(),
                AilFunctionValue {
                    name: input_name,
                    type_name: input_node.type_name.clone().unwrap_or_default(),
                    provenance: node_provenance(core, &input_node.id).unwrap_or_default(),
                },
            );
        }
        for output_node in outgoing_nodes(core, &node_by_id, function_node, "has_output")
            .into_iter()
            .filter(|node| node.kind == "Output")
        {
            let output_name = local_core_name(&output_node.name, &function.name);
            function.outputs.insert(
                output_name.clone(),
                AilFunctionValue {
                    name: output_name,
                    type_name: output_node.type_name.clone().unwrap_or_default(),
                    provenance: node_provenance(core, &output_node.id).unwrap_or_default(),
                },
            );
        }
        function.branches = outgoing_nodes(core, &node_by_id, function_node, "contains")
            .into_iter()
            .filter(|node| node.kind == "Branch")
            .map(|node| {
                node.attributes
                    .get("condition")
                    .cloned()
                    .unwrap_or_else(|| local_core_name(&node.name, &function.name))
            })
            .collect();
        function.calls = outgoing_nodes(core, &node_by_id, function_node, "calls")
            .into_iter()
            .filter(|node| node.kind == "Call")
            .map(|node| AilFunctionCall {
                text: local_core_name(&node.name, &function.name),
                target: node.attributes.get("target").cloned().unwrap_or_default(),
                provenance: node_provenance(core, &node.id).unwrap_or_default(),
            })
            .collect();
        function.termination_bounds =
            outgoing_nodes(core, &node_by_id, function_node, "has_termination_bound")
                .into_iter()
                .filter(|node| node.kind == "TerminationBound")
                .map(|node| {
                    node.attributes
                        .get("value")
                        .cloned()
                        .unwrap_or_else(|| local_core_name(&node.name, &function.name))
                })
                .collect();
        function.termination_measures =
            outgoing_nodes(core, &node_by_id, function_node, "has_termination_measure")
                .into_iter()
                .filter(|node| node.kind == "TerminationMeasure")
                .map(|node| {
                    node.attributes
                        .get("value")
                        .cloned()
                        .unwrap_or_else(|| local_core_name(&node.name, &function.name))
                })
                .collect();
        function.returns = outgoing_nodes(core, &node_by_id, function_node, "contains")
            .into_iter()
            .filter(|node| node.kind == "Return")
            .map(|node| {
                node.attributes
                    .get("value")
                    .cloned()
                    .unwrap_or_else(|| local_core_name(&node.name, &function.name))
            })
            .collect();
        function.traces = outgoing_nodes(core, &node_by_id, function_node, "records_trace")
            .into_iter()
            .filter(|node| node.kind == "Trace")
            .map(|node| node.name)
            .collect();
        document.functions.insert(function.name.clone(), function);
    }

    for binding_node in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "ExternalBinding")
    {
        let mut binding = AilExternalBinding {
            name: binding_node.name.clone(),
            library: binding_node
                .attributes
                .get("library")
                .cloned()
                .unwrap_or_default(),
            symbol: binding_node
                .attributes
                .get("symbol")
                .cloned()
                .unwrap_or_else(|| binding_node.name.clone()),
            binding_kind: binding_node
                .attributes
                .get("binding_kind")
                .cloned()
                .unwrap_or_else(|| "CFunction".to_string()),
            calling_convention: "cdecl".to_string(),
            provenance: node_provenance(core, &binding_node.id).unwrap_or_default(),
            ..AilExternalBinding::default()
        };
        if let Some(layout_node) = outgoing_nodes(core, &node_by_id, binding_node, "uses_layout")
            .into_iter()
            .find(|node| node.kind == "Layout")
            && let Some(type_name) = layout_node.type_name
        {
            binding.calling_convention = type_name;
        }
        for input_node in outgoing_nodes(core, &node_by_id, binding_node, "has_input")
            .into_iter()
            .filter(|node| node.kind == "Input")
        {
            let input_name = local_core_name(&input_node.name, &binding.name);
            binding.inputs.insert(
                input_name.clone(),
                AilExternalBindingValue {
                    name: input_name,
                    type_name: input_node.type_name.clone().unwrap_or_default(),
                    ownership: input_node
                        .attributes
                        .get("ownership")
                        .cloned()
                        .unwrap_or_default(),
                    provenance: node_provenance(core, &input_node.id).unwrap_or_default(),
                },
            );
        }
        for output_node in outgoing_nodes(core, &node_by_id, binding_node, "has_output")
            .into_iter()
            .filter(|node| node.kind == "Output")
        {
            let output_name = local_core_name(&output_node.name, &binding.name);
            binding.outputs.insert(
                output_name.clone(),
                AilExternalBindingValue {
                    name: output_name,
                    type_name: output_node.type_name.clone().unwrap_or_default(),
                    ownership: output_node
                        .attributes
                        .get("ownership")
                        .cloned()
                        .unwrap_or_default(),
                    provenance: node_provenance(core, &output_node.id).unwrap_or_default(),
                },
            );
        }
        binding.status_maps = outgoing_edges(core, binding_node, "may_fail_with")
            .into_iter()
            .filter_map(|edge| {
                let failure = node_by_id.get(&edge.target)?;
                (failure.kind == "Failure").then(|| AilExternalStatusMap {
                    code: edge.attributes.get("code").cloned().unwrap_or_default(),
                    target: format!("Failure.{}", failure.name),
                    provenance: node_provenance(core, &failure.id).unwrap_or_default(),
                })
            })
            .collect();
        binding.status_maps.extend(
            outgoing_edges(core, binding_node, "maps_status")
                .into_iter()
                .filter_map(|edge| {
                    let status = node_by_id.get(&edge.target)?;
                    (status.kind == "StatusMap").then(|| AilExternalStatusMap {
                        code: edge
                            .attributes
                            .get("code")
                            .cloned()
                            .or_else(|| status.attributes.get("code").cloned())
                            .unwrap_or_default(),
                        target: status
                            .type_name
                            .clone()
                            .unwrap_or_else(|| status.name.clone()),
                        provenance: node_provenance(core, &status.id).unwrap_or_default(),
                    })
                }),
        );
        binding.capabilities = outgoing_nodes(core, &node_by_id, binding_node, "requires")
            .into_iter()
            .filter(|node| node.kind == "Capability")
            .map(|node| node.name)
            .collect();
        binding.traces = outgoing_nodes(core, &node_by_id, binding_node, "records_trace")
            .into_iter()
            .filter(|node| node.kind == "Trace")
            .map(|node| node.name)
            .collect();
        document
            .external_bindings
            .insert(binding.name.clone(), binding);
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
            calls: outgoing_nodes(core, &node_by_id, action_node, "calls")
                .into_iter()
                .filter(|node| node.kind == "Action")
                .map(|node| node.name)
                .collect(),
            repeated_calls: outgoing_repeated_action_calls(core, &node_by_id, action_node),
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

fn outgoing_edges(core: &AilCore, source: &Node, edge_kind: &str) -> Vec<Edge> {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == edge_kind && edge.source == source.id)
        .cloned()
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

fn outgoing_repeated_action_calls(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    source: &Node,
) -> Vec<AilRepeatedActionCall> {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "repeats" && edge.source == source.id)
        .filter_map(|edge| {
            let target = node_by_id.get(&edge.target)?;
            (target.kind == "Action").then(|| AilRepeatedActionCall {
                target: target.name.clone(),
                count: edge
                    .attributes
                    .get("count")
                    .and_then(|count| count.parse::<usize>().ok())
                    .unwrap_or(1),
                provenance: format!("action:{}.repeat:{}", source.name, target.name),
            })
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

fn compile_ail_ui_bytecode_actions(document: &AilDocument) -> BTreeMap<String, AilBytecodeAction> {
    let mut actions = document
        .actions
        .iter()
        .map(|(name, action)| (name.clone(), compile_ail_action_bytecode(document, action)))
        .collect::<BTreeMap<_, _>>();
    actions.extend(
        document
            .routes
            .iter()
            .map(|(name, route)| (name.clone(), compile_ail_ui_route_bytecode(document, route))),
    );
    actions.extend(
        document
            .forms
            .iter()
            .map(|(name, form)| (name.clone(), compile_ail_ui_form_bytecode(form))),
    );
    actions.extend(document.dashboards.iter().map(|(name, dashboard)| {
        (
            name.clone(),
            compile_ail_ui_dashboard_bytecode(document, dashboard),
        )
    }));
    actions.extend(
        document
            .workflows
            .iter()
            .map(|(name, workflow)| (name.clone(), compile_ail_ui_workflow_bytecode(workflow))),
    );
    actions
}

fn compile_ail_ui_route_bytecode(document: &AilDocument, route: &AilRoute) -> AilBytecodeAction {
    let mut instructions = vec![AilBytecodeInstruction::new(
        "ACTION_BEGIN",
        &[("action", route.name.clone())],
    )];
    instructions.push(AilBytecodeInstruction::new(
        "OBSERVE_RULE",
        &[("rule", format!("route path {}", route.path))],
    ));
    for read in &route.reads {
        instructions.push(AilBytecodeInstruction::new(
            "READ_FIELD",
            &[
                ("key", ui_runtime_field_key(document, read)),
                ("text", read.clone()),
            ],
        ));
    }
    for permission in &route.permissions {
        instructions.push(AilBytecodeInstruction::new(
            "OBSERVE_RULE",
            &[("rule", format!("permission {permission}"))],
        ));
    }
    for trace in &route.traces {
        instructions.push(AilBytecodeInstruction::new(
            "EMIT_TRACE",
            &[("event", trace.clone())],
        ));
    }
    instructions.push(AilBytecodeInstruction::new("RETURN_SUCCESS", &[]));
    AilBytecodeAction {
        name: route.name.clone(),
        instructions,
    }
}

fn compile_ail_ui_form_bytecode(form: &AilForm) -> AilBytecodeAction {
    let mut instructions = vec![AilBytecodeInstruction::new(
        "ACTION_BEGIN",
        &[("action", form.name.clone())],
    )];
    for field in form.fields.values() {
        instructions.push(AilBytecodeInstruction::new(
            "OBSERVE_RULE",
            &[(
                "rule",
                format!("field {} : {}", field.name, field.type_name),
            )],
        ));
    }
    for validation in &form.validations {
        instructions.push(AilBytecodeInstruction::new(
            "OBSERVE_RULE",
            &[("rule", format!("validation {validation}"))],
        ));
    }
    for confirmation in &form.confirmations {
        instructions.push(AilBytecodeInstruction::new(
            "OBSERVE_RULE",
            &[("rule", format!("confirmation {confirmation}"))],
        ));
    }
    for accessibility in &form.accessibility {
        instructions.push(AilBytecodeInstruction::new(
            "OBSERVE_RULE",
            &[("rule", format!("accessibility {accessibility}"))],
        ));
    }
    if let Some(action) = &form.action {
        instructions.push(AilBytecodeInstruction::new(
            "CALL_ACTION",
            &[("target", action.clone())],
        ));
    }
    for trace in &form.failure_traces {
        instructions.push(AilBytecodeInstruction::new(
            "EMIT_TRACE",
            &[("event", trace.clone())],
        ));
    }
    instructions.push(AilBytecodeInstruction::new("RETURN_SUCCESS", &[]));
    AilBytecodeAction {
        name: form.name.clone(),
        instructions,
    }
}

fn compile_ail_ui_dashboard_bytecode(
    document: &AilDocument,
    dashboard: &AilDashboard,
) -> AilBytecodeAction {
    let mut instructions = vec![AilBytecodeInstruction::new(
        "ACTION_BEGIN",
        &[("action", dashboard.name.clone())],
    )];
    for read in &dashboard.reads {
        instructions.push(AilBytecodeInstruction::new(
            "READ_FIELD",
            &[
                ("key", ui_runtime_field_key(document, read)),
                ("text", read.clone()),
            ],
        ));
    }
    for permission in &dashboard.permissions {
        instructions.push(AilBytecodeInstruction::new(
            "OBSERVE_RULE",
            &[("rule", format!("permission {permission}"))],
        ));
    }
    for filter in &dashboard.filters {
        instructions.push(AilBytecodeInstruction::new(
            "OBSERVE_RULE",
            &[("rule", format!("filter {filter}"))],
        ));
    }
    for trace in &dashboard.traces {
        instructions.push(AilBytecodeInstruction::new(
            "EMIT_TRACE",
            &[("event", trace.clone())],
        ));
    }
    instructions.push(AilBytecodeInstruction::new("RETURN_SUCCESS", &[]));
    AilBytecodeAction {
        name: dashboard.name.clone(),
        instructions,
    }
}

fn compile_ail_ui_workflow_bytecode(workflow: &AilWorkflow) -> AilBytecodeAction {
    let mut instructions = vec![AilBytecodeInstruction::new(
        "ACTION_BEGIN",
        &[("action", workflow.name.clone())],
    )];
    for step in &workflow.steps {
        instructions.push(AilBytecodeInstruction::new(
            "OBSERVE_RULE",
            &[("rule", format!("workflow step {step}"))],
        ));
    }
    for block in &workflow.blocks {
        instructions.push(AilBytecodeInstruction::new(
            "OBSERVE_RULE",
            &[(
                "rule",
                format!(
                    "workflow blocks {} before {}",
                    block.blocked_step, block.prerequisite_step
                ),
            )],
        ));
    }
    for trace in &workflow.traces {
        instructions.push(AilBytecodeInstruction::new(
            "EMIT_TRACE",
            &[("event", trace.clone())],
        ));
    }
    instructions.push(AilBytecodeInstruction::new("RETURN_SUCCESS", &[]));
    AilBytecodeAction {
        name: workflow.name.clone(),
        instructions,
    }
}

fn ui_runtime_field_key(document: &AilDocument, text: &str) -> String {
    referenced_runtime_field_key(document, text).unwrap_or_else(|| {
        format!(
            "ui.{}",
            text.to_ascii_lowercase()
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '.' })
                .collect::<String>()
                .split('.')
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(".")
        )
    })
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

fn compile_ail_function_bytecode(function: &AilFunction) -> AilBytecodeAction {
    let mut instructions = Vec::new();
    instructions.push(AilBytecodeInstruction::new(
        "FUNCTION_BEGIN",
        &[
            ("function", function.name.clone()),
            ("label", function.label.clone()),
        ],
    ));
    for input in function.inputs.values() {
        instructions.push(AilBytecodeInstruction::new(
            "FUNCTION_INPUT",
            &[
                ("name", input.name.clone()),
                ("type", input.type_name.clone()),
            ],
        ));
    }
    for output in function.outputs.values() {
        instructions.push(AilBytecodeInstruction::new(
            "FUNCTION_OUTPUT",
            &[
                ("name", output.name.clone()),
                ("type", output.type_name.clone()),
            ],
        ));
    }
    for branch in &function.branches {
        instructions.push(AilBytecodeInstruction::new(
            "FUNCTION_BRANCH",
            &[("condition", branch.clone())],
        ));
    }
    for call in &function.calls {
        instructions.push(AilBytecodeInstruction::new(
            "FUNCTION_CALL",
            &[("target", call.target.clone()), ("text", call.text.clone())],
        ));
    }
    for return_value in &function.returns {
        instructions.push(AilBytecodeInstruction::new(
            "FUNCTION_RETURN",
            &[("value", return_value.clone())],
        ));
    }
    if function_is_option_map(function) {
        instructions.push(AilBytecodeInstruction::new("OPTION_MAP", &[]));
    }
    for event in &function.traces {
        instructions.push(AilBytecodeInstruction::new(
            "EMIT_TRACE",
            &[("event", event.clone())],
        ));
    }
    instructions.push(AilBytecodeInstruction::new("RETURN_SUCCESS", &[]));
    AilBytecodeAction {
        name: function.name.clone(),
        instructions,
    }
}

fn function_is_option_map(function: &AilFunction) -> bool {
    function.name == "Option.map"
        && (function.calls.iter().any(|call| call.target == "mapper")
            || function
                .branches
                .iter()
                .any(|branch| branch.contains("function calls mapper with value")))
        && function
            .returns
            .iter()
            .any(|return_value| return_value == "Some(mapped value)")
        && function
            .returns
            .iter()
            .any(|return_value| return_value == "None")
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
    for call in &action.calls {
        let target =
            action_call_target_name(document, call).unwrap_or_else(|| action_name_from_label(call));
        instructions.push(AilBytecodeInstruction::new(
            "CALL_ACTION",
            &[("target", target)],
        ));
    }
    for repeated_call in &action.repeated_calls {
        let target = action_call_target_name(document, &repeated_call.target)
            .unwrap_or_else(|| action_name_from_label(&repeated_call.target));
        instructions.push(AilBytecodeInstruction::new(
            "REPEAT_ACTION",
            &[
                ("target", target),
                ("count", repeated_call.count.to_string()),
            ],
        ));
    }
    for write in &action.writes {
        if let Some((key, delta)) = field_integer_delta_assignment(document, write) {
            instructions.push(AilBytecodeInstruction::new(
                "ADD_INT_FIELD",
                &[("key", key), ("delta", delta), ("text", write.clone())],
            ));
        } else if let Some((key, value)) = field_write_assignment(document, write) {
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
        "{{\"kind\":\"AIL-Bytecode\",\"package\":{},\"version\":{},\"profile\":{},\"capability_grants\":{},\"target_support\":{},\"external_bindings\":{},\"actions\":[{}],\"failures\":[{}]}}",
        json_string(&program.package_name),
        json_string(&program.package_version),
        json_string(&program.profile),
        render_ail_bytecode_capability_grants(&program.capability_grants),
        render_json_string_map(&program.target_support),
        render_ail_bytecode_external_bindings(&program.external_bindings),
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

fn render_ail_bytecode_external_bindings(
    bindings: &BTreeMap<String, AilExternalBinding>,
) -> String {
    format!(
        "[{}]",
        bindings
            .values()
            .map(render_ail_bytecode_external_binding)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_ail_bytecode_external_binding(binding: &AilExternalBinding) -> String {
    format!(
        "{{\"name\":{},\"library\":{},\"symbol\":{},\"binding_kind\":{},\"calling_convention\":{},\"inputs\":{},\"outputs\":{},\"status_maps\":{},\"capabilities\":{},\"traces\":{},\"provenance\":{}}}",
        json_string(&binding.name),
        json_string(&binding.library),
        json_string(&binding.symbol),
        json_string(&binding.binding_kind),
        json_string(&binding.calling_convention),
        render_ail_bytecode_external_binding_values(&binding.inputs),
        render_ail_bytecode_external_binding_values(&binding.outputs),
        render_ail_bytecode_external_status_maps(&binding.status_maps),
        render_json_array(binding.capabilities.clone()),
        render_json_array(binding.traces.clone()),
        json_string(&binding.provenance)
    )
}

fn render_ail_bytecode_external_binding_values(
    values: &BTreeMap<String, AilExternalBindingValue>,
) -> String {
    format!(
        "[{}]",
        values
            .values()
            .map(|value| {
                format!(
                    "{{\"name\":{},\"type\":{},\"ownership\":{},\"provenance\":{}}}",
                    json_string(&value.name),
                    json_string(&value.type_name),
                    json_string(&value.ownership),
                    json_string(&value.provenance)
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_ail_bytecode_external_status_maps(status_maps: &[AilExternalStatusMap]) -> String {
    format!(
        "[{}]",
        status_maps
            .iter()
            .map(|status_map| {
                format!(
                    "{{\"code\":{},\"target\":{},\"provenance\":{}}}",
                    json_string(&status_map.code),
                    json_string(&status_map.target),
                    json_string(&status_map.provenance)
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_ail_bytecode_capability_grants(grants: &[AilCapabilityGrant]) -> String {
    format!(
        "[{}]",
        grants
            .iter()
            .map(|grant| {
                format!(
                    "{{\"package\":{},\"capability\":{},\"effects\":{},\"approvals\":{}}}",
                    json_string(&grant.package),
                    json_string(&grant.capability),
                    render_json_array(grant.effects.clone()),
                    render_json_array(grant.approvals.clone())
                )
            })
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
        capability_grants: optional_ail_bytecode_capability_grants(root)?,
        target_support: optional_json_string_map(root, "target_support", "AIL-Bytecode")?,
        external_bindings_metadata_present: root.contains_key("external_bindings"),
        external_bindings: optional_ail_bytecode_external_bindings(root)?,
        actions,
        failures,
    })
}

fn optional_ail_bytecode_external_bindings(
    root: &BTreeMap<String, AilJsonValue>,
) -> Result<BTreeMap<String, AilExternalBinding>, String> {
    let Some(value) = root.get("external_bindings") else {
        return Ok(BTreeMap::new());
    };
    let bindings = value
        .as_array()
        .ok_or_else(|| "AIL-Bytecode field 'external_bindings' must be an array".to_string())?;
    let mut parsed = BTreeMap::new();
    for binding_value in bindings {
        let binding_object = binding_value
            .as_object()
            .ok_or_else(|| "AIL-Bytecode external binding must be an object".to_string())?;
        let name = required_json_string(binding_object, "name")?.to_string();
        parsed.insert(
            name.clone(),
            AilExternalBinding {
                name,
                library: required_json_string(binding_object, "library")?.to_string(),
                symbol: required_json_string(binding_object, "symbol")?.to_string(),
                binding_kind: required_json_string(binding_object, "binding_kind")?.to_string(),
                calling_convention: required_json_string(binding_object, "calling_convention")?
                    .to_string(),
                inputs: required_ail_bytecode_external_binding_values(binding_object, "inputs")?,
                outputs: required_ail_bytecode_external_binding_values(binding_object, "outputs")?,
                status_maps: required_ail_bytecode_external_status_maps(
                    binding_object,
                    "status_maps",
                )?,
                capabilities: required_ail_bytecode_string_array(binding_object, "capabilities")?,
                traces: required_ail_bytecode_string_array(binding_object, "traces")?,
                provenance: required_json_string(binding_object, "provenance")?.to_string(),
            },
        );
    }
    Ok(parsed)
}

fn required_ail_bytecode_external_binding_values(
    object: &BTreeMap<String, AilJsonValue>,
    key: &str,
) -> Result<BTreeMap<String, AilExternalBindingValue>, String> {
    let mut values = BTreeMap::new();
    for value in required_json_array(object, key)? {
        let value_object = value.as_object().ok_or_else(|| {
            format!("AIL-Bytecode external binding field '{key}' entry must be an object")
        })?;
        let name = required_json_string(value_object, "name")?.to_string();
        values.insert(
            name.clone(),
            AilExternalBindingValue {
                name,
                type_name: required_json_string(value_object, "type")?.to_string(),
                ownership: required_json_string(value_object, "ownership")?.to_string(),
                provenance: required_json_string(value_object, "provenance")?.to_string(),
            },
        );
    }
    Ok(values)
}

fn required_ail_bytecode_external_status_maps(
    object: &BTreeMap<String, AilJsonValue>,
    key: &str,
) -> Result<Vec<AilExternalStatusMap>, String> {
    required_json_array(object, key)?
        .iter()
        .map(|value| {
            let status_object = value.as_object().ok_or_else(|| {
                format!("AIL-Bytecode external binding field '{key}' entry must be an object")
            })?;
            Ok(AilExternalStatusMap {
                code: required_json_string(status_object, "code")?.to_string(),
                target: required_json_string(status_object, "target")?.to_string(),
                provenance: required_json_string(status_object, "provenance")?.to_string(),
            })
        })
        .collect()
}

fn optional_ail_bytecode_capability_grants(
    root: &BTreeMap<String, AilJsonValue>,
) -> Result<Vec<AilCapabilityGrant>, String> {
    let Some(value) = root.get("capability_grants") else {
        return Ok(Vec::new());
    };
    let grants = value
        .as_array()
        .ok_or_else(|| "AIL-Bytecode field 'capability_grants' must be an array".to_string())?;
    grants
        .iter()
        .map(|grant_value| {
            let grant = grant_value
                .as_object()
                .ok_or_else(|| "AIL-Bytecode capability grant must be an object".to_string())?;
            Ok(AilCapabilityGrant {
                package: required_json_string(grant, "package")?.to_string(),
                capability: required_json_string(grant, "capability")?.to_string(),
                effects: required_ail_bytecode_string_array(grant, "effects")?,
                approvals: required_ail_bytecode_string_array(grant, "approvals")?,
            })
        })
        .collect()
}

fn required_ail_bytecode_string_array(
    object: &BTreeMap<String, AilJsonValue>,
    key: &str,
) -> Result<Vec<String>, String> {
    required_json_array(object, key)?
        .iter()
        .map(|value| {
            value
                .as_string()
                .map(str::to_string)
                .ok_or_else(|| format!("AIL-Bytecode field '{key}' entries must be strings"))
        })
        .collect()
}

pub fn verify_ail_bytecode(program: &AilBytecodeProgram) -> Vec<String> {
    let mut diagnostics = Vec::new();
    for (target, status) in &program.target_support {
        if !is_known_target_support_status(status) {
            diagnostics.push(format!(
                "AIL-BACKEND-002 unknown AIL target-support status '{status}' for target {target}"
            ));
        }
    }
    for action in program.actions.values() {
        let (labels, label_diagnostics) = ail_bytecode_action_labels(action);
        diagnostics.extend(label_diagnostics);
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
            if matches!(instruction.opcode.as_str(), "BRANCH_FIELD_EQUALS" | "JUMP")
                && let Some(label) = instruction.operands.get("label")
                && !labels.contains_key(label)
            {
                diagnostics.push(format!(
                    "AILBC003 action {} instruction {} targets unknown label {}",
                    action.name, index, label
                ));
            }
            if instruction.opcode == "CALL_ACTION"
                && let Some(target) = instruction.operands.get("target")
                && !program.actions.contains_key(target)
            {
                diagnostics.push(format!(
                    "AILBC005 action {} instruction {} calls unknown action {}",
                    action.name, index, target
                ));
            }
            if instruction.opcode == "REPEAT_ACTION"
                && let Some(target) = instruction.operands.get("target")
                && !program.actions.contains_key(target)
            {
                diagnostics.push(format!(
                    "AILBC005 action {} instruction {} repeats unknown action {}",
                    action.name, index, target
                ));
            }
            if instruction.opcode == "ADD_INT_FIELD"
                && let Some(delta) = instruction.operands.get("delta")
                && delta.parse::<i64>().is_err()
            {
                diagnostics.push(format!(
                    "AILBC006 action {} instruction {} opcode ADD_INT_FIELD delta '{}' is not an integer",
                    action.name, index, delta
                ));
            }
            if instruction.opcode == "REPEAT_ACTION"
                && let Some(count) = instruction.operands.get("count")
                && !count
                    .parse::<usize>()
                    .is_ok_and(|parsed_count| parsed_count > 0)
            {
                diagnostics.push(format!(
                    "AILBC007 action {} instruction {} opcode REPEAT_ACTION count '{}' is not a positive integer",
                    action.name, index, count
                ));
            }
        }
    }
    diagnostics
}

fn ail_bytecode_action_labels(
    action: &AilBytecodeAction,
) -> (BTreeMap<String, usize>, Vec<String>) {
    let mut labels = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for (index, instruction) in action.instructions.iter().enumerate() {
        if instruction.opcode != "LABEL" {
            continue;
        }
        let Some(name) = instruction.operands.get("name") else {
            continue;
        };
        if labels.insert(name.clone(), index).is_some() {
            diagnostics.push(format!(
                "AILBC004 action {} instruction {} duplicates label {}",
                action.name, index, name
            ));
        }
    }
    (labels, diagnostics)
}

fn ail_bytecode_required_operands(opcode: &str) -> Option<&'static [&'static str]> {
    match opcode {
        "ACTION_BEGIN" => Some(&["action"]),
        "LABEL" => Some(&["name"]),
        "BRANCH_FIELD_EQUALS" => Some(&["key", "value", "label"]),
        "JUMP" => Some(&["label"]),
        "CALL_ACTION" => Some(&["target"]),
        "REPEAT_ACTION" => Some(&["target", "count"]),
        "ADD_INT_FIELD" => Some(&["key", "delta", "text"]),
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
        "FUNCTION_BEGIN" => Some(&["function", "label"]),
        "FUNCTION_INPUT" => Some(&["name", "type"]),
        "FUNCTION_OUTPUT" => Some(&["name", "type"]),
        "FUNCTION_BRANCH" => Some(&["condition"]),
        "FUNCTION_CALL" => Some(&["target", "text"]),
        "FUNCTION_RETURN" => Some(&["value"]),
        "OPTION_MAP" => Some(&[]),
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
    run_verified_ail_bytecode_action(program, action_name, runtime_state, 0)
}

fn run_verified_ail_bytecode_action(
    program: &AilBytecodeProgram,
    action_name: &str,
    runtime_state: BTreeMap<String, String>,
    call_depth: usize,
) -> Result<AilRunResult, String> {
    if call_depth > 64 {
        return Err(format!(
            "AIL bytecode action call depth exceeded while calling {action_name}"
        ));
    }
    let action = program
        .actions
        .get(action_name)
        .ok_or_else(|| format!("unknown AIL bytecode action '{action_name}'"))?;
    let mut final_state = runtime_state;
    let mut trace = Vec::new();
    let (labels, _) = ail_bytecode_action_labels(action);
    let mut instruction_pointer = 0usize;
    let mut steps = 0usize;
    let step_limit = action.instructions.len().saturating_mul(1024).max(1024);
    while instruction_pointer < action.instructions.len() {
        steps += 1;
        if steps > step_limit {
            return Err(format!(
                "AIL bytecode execution step limit exceeded in action {action_name}"
            ));
        }
        let instruction = &action.instructions[instruction_pointer];
        match instruction.opcode.as_str() {
            "ACTION_BEGIN" => {
                let action = ail_bytecode_operand(instruction, "action");
                trace.push(format!("action {action} started"));
            }
            "FUNCTION_BEGIN" => {
                trace.push(format!(
                    "function {} started",
                    ail_bytecode_operand(instruction, "label")
                ));
            }
            "FUNCTION_INPUT" => {
                trace.push(format!(
                    "function input {}:{}",
                    ail_bytecode_operand(instruction, "name"),
                    ail_bytecode_operand(instruction, "type")
                ));
            }
            "FUNCTION_OUTPUT" => {
                trace.push(format!(
                    "function output {}:{}",
                    ail_bytecode_operand(instruction, "name"),
                    ail_bytecode_operand(instruction, "type")
                ));
            }
            "FUNCTION_BRANCH" => {
                trace.push(format!(
                    "function branch {}",
                    ail_bytecode_operand(instruction, "condition")
                ));
            }
            "FUNCTION_CALL" => {
                trace.push(format!(
                    "function call {}",
                    ail_bytecode_operand(instruction, "target")
                ));
            }
            "FUNCTION_RETURN" => {
                trace.push(format!(
                    "function return {}",
                    ail_bytecode_operand(instruction, "value")
                ));
            }
            "LABEL" => {}
            "BRANCH_FIELD_EQUALS" => {
                let key = ail_bytecode_operand(instruction, "key");
                let value = ail_bytecode_operand(instruction, "value");
                let label = ail_bytecode_operand(instruction, "label");
                if final_state.get(key).is_some_and(|current| current == value) {
                    trace.push(format!("branch {label} taken"));
                    instruction_pointer = *labels
                        .get(label)
                        .ok_or_else(|| format!("unknown AIL bytecode label '{label}'"))?;
                    continue;
                }
                trace.push(format!("branch {label} skipped"));
            }
            "JUMP" => {
                let label = ail_bytecode_operand(instruction, "label");
                trace.push(format!("jump {label}"));
                instruction_pointer = *labels
                    .get(label)
                    .ok_or_else(|| format!("unknown AIL bytecode label '{label}'"))?;
                continue;
            }
            "CALL_ACTION" => {
                let target = ail_bytecode_operand(instruction, "target");
                trace.push(format!("call action {target}"));
                let mut called =
                    run_verified_ail_bytecode_action(program, target, final_state, call_depth + 1)?;
                trace.append(&mut called.trace);
                if called.status != "succeeded" {
                    return Ok(AilRunResult {
                        status: called.status,
                        failure: called.failure,
                        final_state: called.final_state,
                        trace,
                    });
                }
                final_state = called.final_state;
            }
            "REPEAT_ACTION" => {
                let target = ail_bytecode_operand(instruction, "target");
                let count = ail_bytecode_operand(instruction, "count")
                    .parse::<usize>()
                    .map_err(|_| {
                        format!("REPEAT_ACTION count for '{target}' must be a positive integer")
                    })?;
                if count == 0 {
                    return Err(format!(
                        "REPEAT_ACTION count for '{target}' must be a positive integer"
                    ));
                }
                trace.push(format!("repeat action {target} {count} times"));
                for iteration in 1..=count {
                    trace.push(format!("repeat {target} iteration {iteration}"));
                    let mut called = run_verified_ail_bytecode_action(
                        program,
                        target,
                        final_state,
                        call_depth + 1,
                    )?;
                    trace.append(&mut called.trace);
                    if called.status != "succeeded" {
                        return Ok(AilRunResult {
                            status: called.status,
                            failure: called.failure,
                            final_state: called.final_state,
                            trace,
                        });
                    }
                    final_state = called.final_state;
                }
            }
            "ADD_INT_FIELD" => {
                let key = ail_bytecode_operand(instruction, "key");
                let delta = ail_bytecode_operand(instruction, "delta")
                    .parse::<i64>()
                    .map_err(|_| format!("ADD_INT_FIELD delta for '{key}' must be an integer"))?;
                let current = final_state
                    .get(key)
                    .ok_or_else(|| format!("ADD_INT_FIELD missing integer state field '{key}'"))?
                    .parse::<i64>()
                    .map_err(|_| format!("ADD_INT_FIELD state field '{key}' must be an integer"))?;
                let next = current + delta;
                final_state.insert(key.to_string(), next.to_string());
                trace.push(format!("add {key} by {delta} -> {next}"));
            }
            "OPTION_MAP" => match final_state.get("option.variant").map(String::as_str) {
                Some("Some") => {
                    let mapped = final_state
                        .get("mapper.result")
                        .cloned()
                        .ok_or_else(|| "OPTION_MAP missing mapper.result".to_string())?;
                    final_state.insert("result.variant".to_string(), "Some".to_string());
                    final_state.insert("result.value".to_string(), mapped);
                    trace.push(
                        "option map Some(value) with mapper -> Some(mapped value)".to_string(),
                    );
                }
                Some("None") => {
                    final_state.insert("result.variant".to_string(), "None".to_string());
                    final_state.remove("result.value");
                    trace.push("option map None -> None".to_string());
                }
                Some(other) => {
                    return Err(format!(
                        "OPTION_MAP option.variant must be Some or None, got {other}"
                    ));
                }
                None => {
                    return Err("OPTION_MAP missing option.variant".to_string());
                }
            },
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
        instruction_pointer += 1;
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
    match draft_ail_requirements_response(package, user_prompt, endpoint)? {
        crate::llm::LlmArtifactResponse::Artifact(artifact_text) => Ok(artifact_text),
        crate::llm::LlmArtifactResponse::Questions(questions) => Err(format!(
            "model returned blocking questions:\n- {}",
            questions.join("\n- ")
        )),
    }
}

pub fn draft_ail_requirements_response(
    package: &AilPackage,
    user_prompt: &str,
    endpoint: &str,
) -> Result<crate::llm::LlmArtifactResponse, String> {
    let prompt = build_ail_requirements_prompt(package, user_prompt);
    crate::llm::invoke_llm_artifact_response(
        endpoint,
        &prompt,
        "AIL-Requirements",
        package.metadata.profile.as_str(),
    )
}

pub fn draft_ail_requirements_response_recorded(
    package: &AilPackage,
    user_prompt: &str,
    endpoint: &str,
) -> Result<crate::llm::LlmRecordedArtifactResponse, String> {
    let prompt = build_ail_requirements_prompt(package, user_prompt);
    crate::llm::invoke_llm_artifact_response_recorded(
        endpoint,
        &prompt,
        "AIL-Requirements",
        package.metadata.profile.as_str(),
    )
}

pub fn draft_ail_requirements_response_recorded_with_max_tokens(
    package: &AilPackage,
    user_prompt: &str,
    endpoint: &str,
    max_tokens: usize,
) -> Result<crate::llm::LlmRecordedArtifactResponse, String> {
    let prompt = build_ail_requirements_prompt(package, user_prompt);
    crate::llm::invoke_llm_artifact_response_recorded_with_max_tokens(
        endpoint,
        &prompt,
        "AIL-Requirements",
        package.metadata.profile.as_str(),
        max_tokens,
    )
}

pub fn draft_ail_interview(
    package: &AilPackage,
    user_prompt: &str,
    endpoint: &str,
) -> Result<String, String> {
    let prompt = build_ail_interview_prompt(package, user_prompt);
    match crate::llm::invoke_llm_artifact_response(
        endpoint,
        &prompt,
        "AIL-Interview",
        package.metadata.profile.as_str(),
    )? {
        crate::llm::LlmArtifactResponse::Artifact(artifact_text) => {
            Ok(normalize_ail_interview_artifact(&artifact_text))
        }
        crate::llm::LlmArtifactResponse::Questions(questions) => {
            Ok(render_ail_interview_questions(&questions))
        }
    }
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

pub fn draft_ail_spec_from_requirements_recorded(
    package: &AilPackage,
    user_prompt: &str,
    requirements_text: &str,
    endpoint: &str,
) -> Result<(AilDraftResult, crate::llm::LlmRecordedArtifactResponse), String> {
    let grounded_prompt = format!("{user_prompt}\n\nDRAFT REQUIREMENTS:\n{requirements_text}");
    let prompt = build_ail_draft_prompt(package, &grounded_prompt);
    let recorded = crate::llm::invoke_llm_artifact_response_recorded(
        endpoint,
        &prompt,
        "AIL-Spec Canonical",
        package.metadata.profile.as_str(),
    )?;
    let crate::llm::LlmArtifactResponse::Artifact(spec_text) = &recorded.outcome else {
        return Err("model returned blocking questions for AIL-Spec Canonical".to_string());
    };
    Ok((
        check_ail_draft_spec_with_requirements(package, spec_text.clone(), Some(requirements_text)),
        recorded,
    ))
}

pub fn draft_ail_spec_from_requirements_recorded_with_max_tokens(
    package: &AilPackage,
    user_prompt: &str,
    requirements_text: &str,
    endpoint: &str,
    max_tokens: usize,
) -> Result<(AilDraftResult, crate::llm::LlmRecordedArtifactResponse), String> {
    let grounded_prompt = format!("{user_prompt}\n\nDRAFT REQUIREMENTS:\n{requirements_text}");
    let prompt = build_ail_draft_prompt(package, &grounded_prompt);
    let recorded = crate::llm::invoke_llm_artifact_response_recorded_with_max_tokens(
        endpoint,
        &prompt,
        "AIL-Spec Canonical",
        package.metadata.profile.as_str(),
        max_tokens,
    )?;
    let crate::llm::LlmArtifactResponse::Artifact(spec_text) = &recorded.outcome else {
        return Err("model returned blocking questions for AIL-Spec Canonical".to_string());
    };
    Ok((
        check_ail_draft_spec_with_requirements(package, spec_text.clone(), Some(requirements_text)),
        recorded,
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
                diagnostics.extend(check_ail_spec_preserves_requirement_traces(
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

fn check_ail_spec_preserves_requirement_traces(
    document: &AilDocument,
    requirements_text: &str,
) -> Vec<AilDiagnostic> {
    let mut missing_traces = BTreeSet::new();
    for line in requirements_text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- "))
    {
        let Some(trace_name) = requirement_trace_name(line) else {
            continue;
        };
        let lowered = line.to_ascii_lowercase();
        let action_names = requirement_matching_action_names(document, line);
        if action_names.is_empty() {
            if !document_preserves_trace(document, &trace_name) {
                missing_traces.insert((String::new(), trace_name));
            }
            continue;
        }
        if requirement_mentions_ui_surface(&lowered)
            && document_preserves_trace(document, &trace_name)
        {
            continue;
        }
        for action_name in action_names {
            let Some(action) = document.actions.get(&action_name) else {
                continue;
            };
            if !action_or_linked_ui_surface_preserves_trace(document, action, &trace_name) {
                missing_traces.insert((action_name, trace_name.clone()));
            }
        }
    }
    missing_traces
        .into_iter()
        .map(|(action_name, trace_name)| {
            let (message, repair) = if action_name.is_empty() {
                (
                    format!("spec is missing required trace event {trace_name}"),
                    format!(
                        "Add a trace bullet that records trace event {trace_name} on the relevant action, failure, Route, Form, Dashboard, or Workflow."
                    ),
                )
            } else {
                (
                    format!(
                        "spec is missing required trace event {trace_name} for action {action_name}"
                    ),
                    format!(
                        "Add '- the system records a trace event named {trace_name}' to action {action_name}, or add the trace to a Form that calls {action_name}."
                    ),
                )
            };
            AilDiagnostic::error("AILR013", message).with_repair_suggestion(repair)
        })
        .collect()
}

fn requirement_trace_name(line: &str) -> Option<String> {
    let bullet = trim_sentence(line.strip_prefix("- ").unwrap_or(line).trim());
    let lowered = bullet.to_ascii_lowercase();
    for marker in ["trace event named ", "trace event "] {
        let Some(index) = lowered.find(marker) else {
            continue;
        };
        let trace_text = bullet[index + marker.len()..].trim();
        let trace_name = trace_text
            .split(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ',' | ';' | '.'))
            .next()
            .unwrap_or("")
            .trim();
        if !trace_name.is_empty() {
            return Some(trace_name.to_string());
        }
    }
    None
}

fn requirement_matching_action_names(document: &AilDocument, line: &str) -> Vec<String> {
    let compact_line = compact_requirement_match_text(line);
    document
        .actions
        .values()
        .filter(|action| {
            let compact_name = compact_requirement_match_text(&action.name);
            let compact_label = compact_requirement_match_text(&action.label);
            compact_line.contains(&compact_name) || compact_line.contains(&compact_label)
        })
        .map(|action| action.name.clone())
        .collect()
}

fn requirement_mentions_ui_surface(lowered_requirement: &str) -> bool {
    ["route", "form", "dashboard", "workflow"]
        .iter()
        .any(|surface| lowered_requirement.contains(surface))
}

fn action_or_linked_ui_surface_preserves_trace(
    document: &AilDocument,
    action: &AilAction,
    trace_name: &str,
) -> bool {
    action.traces.iter().any(|trace| trace == trace_name)
        || document.forms.values().any(|form| {
            form.action.as_deref() == Some(action.name.as_str())
                && form.failure_traces.iter().any(|trace| trace == trace_name)
        })
}

fn document_preserves_trace(document: &AilDocument, trace_name: &str) -> bool {
    document
        .actions
        .values()
        .any(|action| action.traces.iter().any(|trace| trace == trace_name))
        || document
            .failures
            .values()
            .any(|failure| failure.traces.iter().any(|trace| trace == trace_name))
        || document
            .tools
            .values()
            .any(|tool| tool.traces.iter().any(|trace| trace == trace_name))
        || document
            .system_components
            .values()
            .any(|component| component.traces.iter().any(|trace| trace == trace_name))
        || document
            .routes
            .values()
            .any(|route| route.traces.iter().any(|trace| trace == trace_name))
        || document
            .forms
            .values()
            .any(|form| form.failure_traces.iter().any(|trace| trace == trace_name))
        || document
            .dashboards
            .values()
            .any(|dashboard| dashboard.traces.iter().any(|trace| trace == trace_name))
        || document
            .workflows
            .values()
            .any(|workflow| workflow.traces.iter().any(|trace| trace == trace_name))
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
        let mut entries = fs::read_dir(&rejected_dir)
            .map_err(|error| format!("failed to read {}: {error}", rejected_dir.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("failed to read {}: {error}", rejected_dir.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();

        let mut spec_paths = entries.clone();
        spec_paths.retain(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".ail-spec.md"))
        });

        for path in spec_paths {
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

        let mut package_paths = entries;
        package_paths.retain(|path| path.is_dir() && path.join("ail-package.md").exists());

        for path in package_paths {
            let fixture = file_name_or_display(&path);
            let diagnostics = match load_ail_package_dir(&path).and_then(|fixture_package| {
                parse_ail_package_document(&fixture_package).map(|document| {
                    check_ail_core_diagnostics(&elaborate_ail_core(&fixture_package, &document))
                })
            }) {
                Ok(diagnostics) => diagnostics,
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

#[derive(Default)]
struct AilCapabilityGrantBuilder {
    package: Option<String>,
    capability: Option<String>,
    effects: Vec<String>,
    approvals: Vec<String>,
}

fn parse_package_metadata(text: &str) -> Result<AilPackageMetadata, String> {
    let mut values = BTreeMap::new();
    let mut target_support = BTreeMap::new();
    let mut capability_grants = Vec::new();
    let mut capability_grant = None;
    let mut active_block = "";
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let is_indented = raw_line.starts_with(' ') || raw_line.starts_with('\t');
        if is_indented && active_block == "target-support" {
            let Some((target, status)) = line.split_once(':') else {
                return Err(format!(
                    "AIL target-support entry '{line}' must use '<target>: <status>'"
                ));
            };
            let target = target.trim();
            let status = status.trim();
            if target.is_empty() || status.is_empty() {
                return Err(format!(
                    "AIL target-support entry '{line}' must use '<target>: <status>'"
                ));
            }
            target_support.insert(target.to_string(), status.to_string());
            continue;
        }
        if is_indented && active_block == "capability-grants" {
            parse_capability_grant_manifest_line(
                line,
                &mut capability_grant,
                &mut capability_grants,
            )?;
            continue;
        }
        if active_block == "capability-grants" {
            finish_capability_grant(capability_grant.take(), &mut capability_grants)?;
        }
        active_block = "";
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().to_string();
        if key == "target-support" {
            if value.is_empty() {
                active_block = "target-support";
            } else {
                target_support = parse_target_support_specs(&value)?;
            }
        } else if key == "capability-grants" {
            if value.is_empty() {
                active_block = "capability-grants";
            } else {
                capability_grants = parse_capability_grant_specs(&value)?;
            }
        } else {
            values.insert(key, value);
        }
    }
    if active_block == "capability-grants" {
        finish_capability_grant(capability_grant.take(), &mut capability_grants)?;
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
    let prompt_pack = values.get("prompt-pack").cloned();
    let registry = values.get("registry").cloned();
    let schema_version = values.get("schema-version").cloned();
    let safety_level = values.get("safety-level").cloned();
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
        capability_grants,
        conformance,
        prompt_pack,
        registry,
        target_support,
        schema_version,
        safety_level,
        base_llm_endpoint,
    })
}

fn parse_import_specs(text: &str) -> Result<Vec<AilImportSpec>, String> {
    let mut aliases = BTreeSet::new();
    let mut imports = Vec::new();
    for entry in text
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let Some((path, alias)) = entry.split_once(" as ") else {
            return Err(format!("AIL import '{entry}' must use '<path> as <Alias>'"));
        };
        let path = path.trim();
        let (alias, resolved_package) = match alias.trim().rsplit_once(" resolved ") {
            Some((alias, resolved_package))
                if !alias.trim().is_empty() && !resolved_package.trim().is_empty() =>
            {
                (alias.trim(), Some(resolved_package.trim().to_string()))
            }
            Some(_) => {
                return Err(format!(
                    "AIL import '{entry}' must use '<path> as <Alias> resolved <Package>'"
                ));
            }
            None => (alias.trim(), None),
        };
        let (alias, registry_identity) = match alias.rsplit_once(" registry ") {
            Some((alias, registry_identity))
                if !alias.trim().is_empty() && !registry_identity.trim().is_empty() =>
            {
                (alias.trim(), Some(registry_identity.trim().to_string()))
            }
            Some(_) => {
                return Err(format!(
                    "AIL import '{entry}' must use '<path> as <Alias> registry <Identity>'"
                ));
            }
            None => (alias, None),
        };
        if path.is_empty() || alias.is_empty() {
            return Err(format!("AIL import '{entry}' must use '<path> as <Alias>'"));
        }
        if !aliases.insert(alias.to_string()) {
            return Err(format!("AIL import duplicate import alias {alias}"));
        }
        let (path, version) = if let Some((path, requirement)) = path.split_once(" compatible ") {
            if path.trim().is_empty() || requirement.trim().is_empty() {
                return Err(format!(
                    "AIL import '{entry}' must use '<path> compatible <range> as <Alias>'"
                ));
            }
            let requirement = requirement.trim();
            if import_version_range_is_unbounded_major(requirement) {
                return Err(format!(
                    "AIL import '{entry}' uses unbounded major version range {requirement}"
                ));
            }
            (path.trim(), Some(format!("compatible {requirement}")))
        } else {
            match path.rsplit_once('@') {
                Some((path, version)) if !path.is_empty() && !version.is_empty() => {
                    (path, Some(version.to_string()))
                }
                Some(_) => {
                    return Err(format!(
                        "AIL import '{entry}' must use '<path>@<version> as <Alias>'"
                    ));
                }
                None => (path, None),
            }
        };
        imports.push(AilImportSpec {
            path: path.to_string(),
            version,
            alias: alias.to_string(),
            resolved_package,
            registry_identity,
        });
    }
    Ok(imports)
}

fn import_version_range_is_unbounded_major(requirement: &str) -> bool {
    matches!(requirement.trim(), "*" | "x" | "X")
}

fn import_version_requirement_matches(requirement: &str, actual: &str) -> Result<bool, String> {
    let Some(range) = requirement.strip_prefix("compatible ") else {
        return Ok(actual == requirement);
    };
    let range = range.trim();
    if import_version_range_is_unbounded_major(range) {
        return Err(format!(
            "AIL import compatible range {range} is unbounded major"
        ));
    }
    let Some(base) = range.strip_prefix('^') else {
        return Err(format!(
            "AIL import compatible range {range} must use caret syntax like ^1.2"
        ));
    };
    let base = parse_semver_prefix(base)?;
    let actual = parse_semver_prefix(actual)?;
    Ok(actual.major == base.major && semver_tuple_at_least(actual, base))
}

#[derive(Clone, Copy)]
struct AilSemverPrefix {
    major: u64,
    minor: u64,
    patch: u64,
}

fn parse_semver_prefix(version: &str) -> Result<AilSemverPrefix, String> {
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 3 {
        return Err(format!("AIL version '{version}' must be major.minor.patch"));
    }
    let major = parse_semver_component(version, parts[0], "major")?;
    let minor = parts
        .get(1)
        .map(|part| parse_semver_component(version, part, "minor"))
        .transpose()?
        .unwrap_or(0);
    let patch = parts
        .get(2)
        .map(|part| parse_semver_component(version, part, "patch"))
        .transpose()?
        .unwrap_or(0);
    Ok(AilSemverPrefix {
        major,
        minor,
        patch,
    })
}

fn parse_semver_component(version: &str, part: &str, name: &str) -> Result<u64, String> {
    if part.is_empty() {
        return Err(format!(
            "AIL version '{version}' has empty {name} component"
        ));
    }
    part.parse::<u64>()
        .map_err(|_| format!("AIL version '{version}' has non-integer {name} component"))
}

fn semver_tuple_at_least(actual: AilSemverPrefix, base: AilSemverPrefix) -> bool {
    (actual.major, actual.minor, actual.patch) >= (base.major, base.minor, base.patch)
}

fn parse_capability_grant_manifest_line(
    line: &str,
    current: &mut Option<AilCapabilityGrantBuilder>,
    grants: &mut Vec<AilCapabilityGrant>,
) -> Result<(), String> {
    let field = if let Some(field) = line.strip_prefix("- ") {
        finish_capability_grant(current.take(), grants)?;
        field.trim()
    } else {
        line
    };
    let Some((key, value)) = field.split_once(':') else {
        return Err(format!(
            "AIL capability-grants entry '{line}' must use '<field>: <value>'"
        ));
    };
    let key = key.trim();
    let value = value.trim();
    if value.is_empty() {
        return Err(format!(
            "AIL capability-grants entry '{line}' must use '<field>: <value>'"
        ));
    }
    let grant = current.get_or_insert_with(AilCapabilityGrantBuilder::default);
    match key {
        "package" => grant.package = Some(value.to_string()),
        "capability" => grant.capability = Some(value.to_string()),
        "effects" => grant.effects = parse_metadata_list(value)?,
        "approvals" => grant.approvals = parse_metadata_list(value)?,
        key => {
            return Err(format!("unknown AIL capability-grants field '{key}'"));
        }
    }
    Ok(())
}

fn finish_capability_grant(
    grant: Option<AilCapabilityGrantBuilder>,
    grants: &mut Vec<AilCapabilityGrant>,
) -> Result<(), String> {
    let Some(grant) = grant else {
        return Ok(());
    };
    let package = grant
        .package
        .ok_or_else(|| "AIL capability-grants entry missing package".to_string())?;
    let capability = grant
        .capability
        .ok_or_else(|| "AIL capability-grants entry missing capability".to_string())?;
    grants.push(AilCapabilityGrant {
        package,
        capability,
        effects: grant.effects,
        approvals: grant.approvals,
    });
    Ok(())
}

fn parse_metadata_list(value: &str) -> Result<Vec<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(Vec::new());
    }
    if value.starts_with('[') || value.ends_with(']') {
        if !(value.starts_with('[') && value.ends_with(']')) {
            return Err(format!(
                "AIL metadata list '{value}' must use '[item, item]'"
            ));
        }
        let inner = value.trim_start_matches('[').trim_end_matches(']');
        return Ok(inner
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToString::to_string)
            .collect());
    }
    Ok(vec![value.to_string()])
}

fn parse_capability_grant_specs(text: &str) -> Result<Vec<AilCapabilityGrant>, String> {
    let mut grants = Vec::new();
    for entry in text
        .split(" || ")
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let mut grant = AilCapabilityGrantBuilder::default();
        for field in entry
            .split(';')
            .map(str::trim)
            .filter(|field| !field.is_empty())
        {
            let Some((key, value)) = field.split_once('=') else {
                return Err(format!(
                    "AIL capability-grants field '{field}' must use '<field>=<value>'"
                ));
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "package" => grant.package = Some(value.to_string()),
                "capability" => grant.capability = Some(value.to_string()),
                "effects" => grant.effects = parse_pipe_list(value),
                "approvals" => grant.approvals = parse_pipe_list(value),
                key => {
                    return Err(format!("unknown AIL capability-grants field '{key}'"));
                }
            }
        }
        finish_capability_grant(Some(grant), &mut grants)?;
    }
    Ok(grants)
}

fn parse_pipe_list(value: &str) -> Vec<String> {
    value
        .split('|')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parse_target_support_specs(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut target_support = BTreeMap::new();
    for entry in text
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let Some((target, status)) = entry.split_once('=') else {
            return Err(format!(
                "AIL target-support entry '{entry}' must use '<target>=<status>'"
            ));
        };
        let target = target.trim();
        let status = status.trim();
        if target.is_empty() || status.is_empty() {
            return Err(format!(
                "AIL target-support entry '{entry}' must use '<target>=<status>'"
            ));
        }
        target_support.insert(target.to_string(), status.to_string());
    }
    Ok(target_support)
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
    for (name, function) in imported.functions {
        target.functions.insert(name, function);
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
        functions: BTreeMap::new(),
        types: BTreeMap::new(),
        routes: BTreeMap::new(),
        forms: BTreeMap::new(),
        dashboards: BTreeMap::new(),
        workflows: BTreeMap::new(),
        external_bindings: BTreeMap::new(),
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
                calls: action
                    .calls
                    .iter()
                    .map(|text| {
                        action_call_target_name(document, text)
                            .map(|target| qualify_name(alias, &target))
                            .unwrap_or_else(|| qualify_reference_text(text, alias, &thing_names))
                    })
                    .collect(),
                repeated_calls: action
                    .repeated_calls
                    .iter()
                    .map(|call| AilRepeatedActionCall {
                        target: action_call_target_name(document, &call.target)
                            .map(|target| qualify_name(alias, &target))
                            .unwrap_or_else(|| {
                                qualify_reference_text(&call.target, alias, &thing_names)
                            }),
                        count: call.count,
                        provenance: format!("action:{action_name}.repeat:{}", call.target),
                    })
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

    for type_decl in document.types.values() {
        let type_name = qualify_name(alias, &type_decl.name);
        namespaced.types.insert(
            type_name.clone(),
            AilType {
                name: type_name.clone(),
                label: qualify_name(alias, &type_decl.label),
                variants: namespace_variants(alias, &type_name, &type_decl.variants, &thing_names),
                provenance: format!("type:{type_name}"),
            },
        );
    }

    for route in document.routes.values() {
        let route_name = qualify_name(alias, &route.name);
        namespaced.routes.insert(
            route_name.clone(),
            AilRoute {
                name: route_name.clone(),
                label: format!("{alias}.{}", route.label),
                path: route.path.clone(),
                reads: route
                    .reads
                    .iter()
                    .map(|read| qualify_reference_text(read, alias, &thing_names))
                    .collect(),
                permissions: route.permissions.clone(),
                traces: route
                    .traces
                    .iter()
                    .map(|trace| qualify_name(alias, trace))
                    .collect(),
                provenance: format!("route:{route_name}"),
            },
        );
    }

    for function in document.functions.values() {
        let function_name = qualify_name(alias, &function.name);
        namespaced.functions.insert(
            function_name.clone(),
            AilFunction {
                name: function_name.clone(),
                label: qualify_name(alias, &function.label),
                inputs: namespace_function_values(
                    alias,
                    &function_name,
                    &function.inputs,
                    &thing_names,
                ),
                outputs: namespace_function_values(
                    alias,
                    &function_name,
                    &function.outputs,
                    &thing_names,
                ),
                branches: function.branches.clone(),
                calls: function
                    .calls
                    .iter()
                    .map(|call| AilFunctionCall {
                        text: call.text.clone(),
                        target: qualify_name(alias, &call.target),
                        provenance: format!("function:{function_name}.call:{}", call.text),
                    })
                    .collect(),
                termination_bounds: function.termination_bounds.clone(),
                termination_measures: function.termination_measures.clone(),
                returns: function.returns.clone(),
                traces: function
                    .traces
                    .iter()
                    .map(|trace| qualify_name(alias, trace))
                    .collect(),
                provenance: format!("function:{function_name}"),
            },
        );
    }

    for binding in document.external_bindings.values() {
        let binding_name = qualify_name(alias, &binding.name);
        namespaced.external_bindings.insert(
            binding_name.clone(),
            AilExternalBinding {
                name: binding_name.clone(),
                library: binding.library.clone(),
                symbol: binding.symbol.clone(),
                binding_kind: binding.binding_kind.clone(),
                calling_convention: binding.calling_convention.clone(),
                inputs: namespace_external_binding_values(
                    alias,
                    &binding_name,
                    &binding.inputs,
                    &thing_names,
                ),
                outputs: namespace_external_binding_values(
                    alias,
                    &binding_name,
                    &binding.outputs,
                    &thing_names,
                ),
                status_maps: binding.status_maps.clone(),
                capabilities: binding.capabilities.clone(),
                traces: binding
                    .traces
                    .iter()
                    .map(|trace| qualify_name(alias, trace))
                    .collect(),
                provenance: format!("external_binding:{binding_name}"),
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

fn namespace_function_values(
    alias: &str,
    function_name: &str,
    values: &BTreeMap<String, AilFunctionValue>,
    thing_names: &[String],
) -> BTreeMap<String, AilFunctionValue> {
    values
        .values()
        .map(|value| {
            (
                value.name.clone(),
                AilFunctionValue {
                    name: value.name.clone(),
                    type_name: qualify_type_name(&value.type_name, alias, thing_names),
                    provenance: format!("function:{function_name}.value:{}", value.name),
                },
            )
        })
        .collect()
}

fn namespace_variants(
    alias: &str,
    type_name: &str,
    variants: &BTreeMap<String, AilVariant>,
    thing_names: &[String],
) -> BTreeMap<String, AilVariant> {
    variants
        .values()
        .map(|variant| {
            (
                variant.name.clone(),
                AilVariant {
                    name: variant.name.clone(),
                    label: variant.label.clone(),
                    fields: namespace_variant_fields(alias, type_name, variant, thing_names),
                    provenance: format!("type:{type_name}.variant:{}", variant.name),
                },
            )
        })
        .collect()
}

fn namespace_variant_fields(
    alias: &str,
    type_name: &str,
    variant: &AilVariant,
    thing_names: &[String],
) -> BTreeMap<String, AilVariantField> {
    variant
        .fields
        .values()
        .map(|field| {
            (
                field.name.clone(),
                AilVariantField {
                    name: field.name.clone(),
                    type_name: qualify_type_name(&field.type_name, alias, thing_names),
                    provenance: format!(
                        "type:{type_name}.variant:{}.field:{}",
                        variant.name, field.name
                    ),
                },
            )
        })
        .collect()
}

fn namespace_external_binding_values(
    alias: &str,
    binding_name: &str,
    values: &BTreeMap<String, AilExternalBindingValue>,
    thing_names: &[String],
) -> BTreeMap<String, AilExternalBindingValue> {
    values
        .values()
        .map(|value| {
            (
                value.name.clone(),
                AilExternalBindingValue {
                    name: value.name.clone(),
                    type_name: qualify_type_name(&value.type_name, alias, thing_names),
                    ownership: value.ownership.clone(),
                    provenance: format!("external_binding:{binding_name}.value:{}", value.name),
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
    for wrapper in ["Secret", "List", "Option", "Pointer", "Nullable", "NonNull"] {
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

fn prompt_envelope_instruction(artifact_kind: &str, expected_profile: &str) -> String {
    format!(
        concat!(
            "Prefer the prompt-pack JSON envelope with ",
            "\"artifact_kind\":\"{}\", \"artifact_text\":\"<artifact>\", ",
            "\"questions\":[], \"assumptions\":[], \"provenance\":[], and ",
            "\"checker_handoff\":{{\"must_check\":true,\"expected_profile\":\"{}\",\"expected_features\":[]}}. ",
            "If required semantics are missing, return an empty artifact_text and focused blocking questions instead of guessing. ",
            "The prompt envelope must contain exactly one mode: either non-empty artifact_text with questions set to [], or empty artifact_text with non-empty questions. ",
            "Never return artifact_text and blocking questions together."
        ),
        artifact_kind, expected_profile
    )
}

const INTERVIEW_SYSTEM_PROMPT: &str = include_str!("../docs/ail/prompts/interview.system.md");
const REQUIREMENTS_SYSTEM_PROMPT: &str = include_str!("../docs/ail/prompts/requirements.system.md");
const SPEC_DRAFT_SYSTEM_PROMPT: &str = include_str!("../docs/ail/prompts/spec-draft.system.md");

fn prompt_pack_header_value<'a>(prompt_text: &'a str, prefix: &str) -> Option<&'a str> {
    prompt_text
        .lines()
        .find_map(|line| line.strip_prefix(prefix).map(str::trim))
        .filter(|value| !value.is_empty())
}

fn prompt_pack_source_block(file_name: &str, prompt_text: &str) -> String {
    let prompt_name = prompt_pack_header_value(prompt_text, "# Prompt:")
        .unwrap_or(file_name.trim_end_matches(".md"));
    let prompt_version = prompt_pack_header_value(prompt_text, "version:").unwrap_or("unknown");
    let prompt_fingerprint = ail_text_fingerprint(prompt_text);
    format!(
        concat!(
            "PROMPT PACK SOURCE {}:\n",
            "prompt_file: {}\n",
            "prompt: {}\n",
            "prompt_version: {}\n",
            "prompt_fingerprint: {}\n",
            "{}\n"
        ),
        file_name, file_name, prompt_name, prompt_version, prompt_fingerprint, prompt_text
    )
}

fn build_ail_interview_prompt(package: &AilPackage, user_prompt: &str) -> String {
    let prompt_pack_source =
        prompt_pack_source_block("interview.system.md", INTERVIEW_SYSTEM_PROMPT);
    format!(
        concat!(
            "Interview the user intent for package {} before drafting AIL requirements.\n",
            "Use the {} profile and conformance level {}.\n",
            "Package features: {}.\n",
            "{}\n",
            "{}\n",
            "Return only a prompt-pack JSON envelope. If required actors, effects, secrets, permissions, failures, guarantees, traces, or runtime inputs are missing, set artifact_text to an empty string and return focused blocking questions. Do not emit AIL-Spec or backend source.\n\n",
            "HUMAN REQUEST:\n",
            "{}\n"
        ),
        package.metadata.name,
        package.metadata.profile,
        package.metadata.conformance,
        package.metadata.features.join(", "),
        prompt_pack_source,
        prompt_envelope_instruction("AIL-Interview", &package.metadata.profile),
        user_prompt
    )
}

fn render_ail_interview_questions(questions: &[String]) -> String {
    let mut lines = vec!["AIL-Interview:".to_string()];
    for question in questions {
        lines.push(format!("- {}", question.trim()));
    }
    lines.join("\n")
}

pub fn render_ail_interview_questions_artifact(questions: &[String]) -> String {
    render_ail_interview_questions(questions)
}

fn normalize_ail_interview_artifact(artifact_text: &str) -> String {
    let artifact_text = artifact_text.trim();
    if artifact_text.starts_with("AIL-Interview:") || artifact_text.starts_with("AIL-Requirements:")
    {
        return artifact_text.to_string();
    }
    format!("AIL-Interview:\n- {artifact_text}")
}

fn build_ail_requirements_prompt(package: &AilPackage, user_prompt: &str) -> String {
    let coverage = ail_requirements_prompt_coverage(&package.metadata.profile);
    let prompt_pack_source =
        prompt_pack_source_block("requirements.system.md", REQUIREMENTS_SYSTEM_PROMPT);
    format!(
        concat!(
            "Draft AIL requirements for package {}.\n",
            "Use the {} profile and conformance level {}.\n",
            "Package features: {}.\n",
            "{}\n",
            "Return only a prompt-pack JSON envelope for AIL-Requirements. Put the complete AIL-Requirements artifact in artifact_text; do not return raw artifact text, code fences, AIL-Spec, implementation code, backend source, or reasoning.\n",
            "Inside artifact_text, the first line must be exactly AIL-Requirements:. Return at least six requirement bullets, and every requirement bullet must start with \"- \"; do not use \"*\" bullets, numbered lists, tables, or Markdown emphasis.\n",
            "{}\n",
            "{}\n",
            "These requirements are an intermediate artifact. The next compiler step will transform them into AIL-Spec, then AIL-Core, then AIL-Bytecode.\n\n",
            "HUMAN REQUEST:\n",
            "{}\n"
        ),
        package.metadata.name,
        package.metadata.profile,
        package.metadata.conformance,
        package.metadata.features.join(", "),
        prompt_pack_source,
        prompt_envelope_instruction("AIL-Requirements", &package.metadata.profile),
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
            "Requirements must name application domain objects, required fields, actions, failure cases, guarantees, traces, source-declared secrets, source-declared permissions, and runtime inputs that the final checked AIL application must preserve. Do not invent secrets, permissions, failures, or runtime inputs that are absent from the package source; when a category is not present, state the absence explicitly instead of adding new semantics. If the package source does not declare Secret<...>, do not introduce secret fields or redaction guarantees."
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
    let prompt_pack_source =
        prompt_pack_source_block("requirements.system.md", REQUIREMENTS_SYSTEM_PROMPT);
    format!(
        concat!(
            "Repair AIL requirements for package {}.\n",
            "Use the {} profile and conformance level {}.\n",
            "{}\n",
            "Return only a prompt-pack JSON envelope for AIL-Requirements. Put the complete repaired AIL-Requirements artifact in artifact_text; do not return raw artifact text, code fences, AIL-Spec, implementation code, backend source, or reasoning.\n",
            "Inside artifact_text, the first line must be exactly AIL-Requirements:. Return at least six requirement bullets, and every requirement bullet must start with \"- \"; do not use \"*\" bullets, numbered lists, tables, or Markdown emphasis.\n",
            "{}\n",
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
        prompt_pack_source,
        prompt_envelope_instruction("AIL-Requirements", &package.metadata.profile),
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
    let prompt_pack_source =
        prompt_pack_source_block("spec-draft.system.md", SPEC_DRAFT_SYSTEM_PROMPT);
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
    } else if package.metadata.profile == "UI"
        || package.metadata.features.iter().any(|feature| {
            matches!(
                feature.as_str(),
                "routes" | "forms" | "dashboards" | "workflows" | "accessibility"
            )
        })
    {
        concat!(
            "Use this exact UI profile surface shape:\n",
            "The application <Name> manages <purpose>.\n\n",
            "A <Thing> has:\n\n",
            "- <field>: <Type>\n\n",
            "Action: <human label>.\n\n",
            "When <trigger>:\n\n",
            "- the system requires <rule>\n",
            "- the system reads <field or effect>\n",
            "- the system changes <field or effect>\n",
            "- the system guarantees <guarantee>\n",
            "- the system records a trace event named <TraceName>\n\n",
            "Route: <human label>.\n\n",
            "The route path is:\n\n",
            "- <path>\n\n",
            "The route reads:\n\n",
            "- <Thing.field>\n\n",
            "The route requires permission:\n\n",
            "- <permission rule>\n\n",
            "The route records trace:\n\n",
            "- <TraceName>\n\n",
            "Form: <human label>.\n\n",
            "The form calls action:\n\n",
            "- <ActionName>\n\n",
            "The form fields are:\n\n",
            "- <field>: <Type>\n\n",
            "The form validates:\n\n",
            "- <validation rule>\n\n",
            "If form validation fails:\n\n",
            "- <TraceName>\n\n",
            "The form accessibility is:\n\n",
            "- <accessibility rule>\n\n",
            "Dashboard: <human label>.\n\n",
            "The dashboard reads:\n\n",
            "- <Thing.field>\n\n",
            "The dashboard requires permission:\n\n",
            "- <permission rule>\n\n",
            "The dashboard filters:\n\n",
            "- <filter rule>\n\n",
            "The dashboard records trace:\n\n",
            "- <TraceName>\n\n",
            "Workflow: <human label>.\n\n",
            "The workflow steps are:\n\n",
            "- <step>\n\n",
            "The workflow blocks:\n\n",
            "- <blocked step> before <prerequisite step>\n\n",
            "The workflow records trace:\n\n",
            "- <TraceName>\n\n",
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
            "- the system does not reveal <secret field> to <audience> (only when the source or requirements define Secret<...>)\n",
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
            "{}\n",
            "Output only parseable AIL-Spec structured English. Do not include code fences, Markdown commentary, labels like Application:, or reasoning.\n",
            "{}\n",
            "The checker will decide whether the candidate is accepted, so preserve explicit things, fields, tools, actions, system components, capabilities, failures, guarantees, traces, and secret handling.\n\n",
            "Use canonical AIL type spellings: Text, State<Open, Closed>, List<Text>, Option<Text>, and Secret<List<Text>> for a secret list of text values.\n\n",
            "Do not emit a secret-redaction line unless the checked requirements or package source define Secret<...>. If no source secret exists, omit that line entirely; do not invent internal tokens, credentials, notes, or private data.\n\n",
            "{}\n\n",
            "HUMAN REQUEST:\n",
            "{}\n"
        ),
        package.metadata.name,
        package.metadata.profile,
        package.metadata.conformance,
        package.metadata.features.join(", "),
        prompt_pack_source,
        prompt_envelope_instruction("AIL-Spec Canonical", &package.metadata.profile),
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
        .or_else(|| requirement.split_once(" not "))
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
    if requirement.contains(" not to be ")
        || requirement.contains(" is not ")
        || requirement.contains(" not ")
    {
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

fn field_integer_delta_assignment(document: &AilDocument, write: &str) -> Option<(String, String)> {
    let (sign, rest) = write
        .strip_prefix("increments ")
        .map(|rest| (1_i64, rest))
        .or_else(|| write.strip_prefix("decrements ").map(|rest| (-1_i64, rest)))?;
    let (field_text, delta_text) = rest.rsplit_once(" by ")?;
    let key = referenced_runtime_field_key(document, field_text.trim_start_matches("the "))?;
    let delta = delta_text
        .split(|ch: char| !ch.is_ascii_digit() && ch != '-')
        .next()
        .unwrap_or("")
        .parse::<i64>()
        .ok()?;
    Some((key, (sign * delta).to_string()))
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
            if contains_reference_phrase(&normalized, &qualified) {
                qualified_matches.push((qualified.len(), key.clone()));
            } else if contains_reference_phrase(&normalized, &field_text) {
                field_matches.push(key.clone());
            }
            if let Some(target_thing) = referenced_thing_type(document, &field.type_name) {
                for nested_field in target_thing.fields.values() {
                    let nested_field_text = nested_field.name.to_ascii_lowercase();
                    let nested_field_phrase = format!("{field_text} {nested_field_text}");
                    let qualified_nested_field_phrase =
                        format!("{thing_text} {nested_field_phrase}");
                    let nested_key = format!("{key}.{}", runtime_subject_key(&nested_field.name));
                    if contains_reference_phrase(&normalized, &qualified_nested_field_phrase) {
                        nested_matches.push((qualified_nested_field_phrase.len(), nested_key));
                    } else if contains_reference_phrase(&normalized, &nested_field_phrase) {
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
    if field_matches.len() > 1
        && let Some(ticket_key) = field_matches
            .iter()
            .find(|key| key.starts_with("ticket."))
            .cloned()
    {
        return Some(ticket_key);
    }
    (field_matches.len() == 1).then(|| field_matches.remove(0))
}

fn contains_reference_phrase(text: &str, phrase: &str) -> bool {
    if phrase.is_empty() {
        return false;
    }
    let mut search_start = 0usize;
    while let Some(offset) = text[search_start..].find(phrase) {
        let start = search_start + offset;
        let end = start + phrase.len();
        if reference_boundary_at(text, start) && reference_boundary_at(text, end) {
            return true;
        }
        search_start = end;
    }
    false
}

fn reference_boundary_at(text: &str, index: usize) -> bool {
    if index == 0 || index >= text.len() {
        return true;
    }
    let before = text[..index].chars().next_back();
    let after = text[index..].chars().next();
    before.is_none_or(|ch| !is_reference_word_char(ch))
        || after.is_none_or(|ch| !is_reference_word_char(ch))
}

fn is_reference_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
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
    } else if let Some(write) = line.strip_prefix("records ") {
        action.writes.push(trim_sentence(write));
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
        .filter(|node| matches!(node.kind.as_str(), "Thing" | "Type"))
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
                    "AIL-TYPE-001",
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
    for wrapper in ["Option", "List", "Secret", "Pointer", "Nullable", "NonNull"] {
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
        if action.kind == "ExternalBinding" {
            continue;
        }
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

fn check_secret_internal_notes_role_requirements(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "reads") {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" {
            continue;
        }
        let Some(target) = node_by_id.get(&edge.target) else {
            continue;
        };
        if target.kind != "Field"
            || !target
                .name
                .to_ascii_lowercase()
                .ends_with(".internal notes")
            || target
                .attributes
                .get("secret")
                .is_none_or(|value| value != "true")
        {
            continue;
        }
        if action_has_secret_support_role_requirement(core, &node_by_id, action) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-SECRET-ROLE-001",
                format!(
                    "action {} reads {} without a support-role requirement",
                    action.name, target.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &action.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add a requirement such as 'the requester role to be SupportAgent or SupportManager' to action {}.",
                action.name
            )),
        );
    }
    diagnostics
}

fn action_has_secret_support_role_requirement(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> bool {
    outgoing_nodes(core, node_by_id, action, "requires")
        .into_iter()
        .filter(|node| node.kind == "Rule")
        .any(|rule| {
            let compact = compact_semantic_text(&rule.name);
            compact.contains("role")
                && (compact.contains("supportagent")
                    || compact.contains("supportmanager")
                    || compact.contains("supportstaff"))
        })
}

fn check_application_assignment_role_requirements(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "writes") {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" {
            continue;
        }
        let Some(target) = node_by_id.get(&edge.target) else {
            continue;
        };
        if target.kind != "Field" || !is_assignee_field(&target.name) {
            continue;
        }
        if action_has_support_role_requirement(core, &node_by_id, action) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-APP-001",
                format!(
                    "action {} writes {} without a support-role requirement",
                    action.name, target.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &action.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add a requirement such as 'the assignee role to be SupportAgent or SupportManager' to action {}.",
                action.name
            )),
        );
    }
    diagnostics
}

fn is_assignee_field(field_name: &str) -> bool {
    let normalized = field_name.to_ascii_lowercase();
    normalized == "assignee" || normalized.ends_with(".assignee")
}

fn action_has_support_role_requirement(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> bool {
    outgoing_nodes(core, node_by_id, action, "requires")
        .into_iter()
        .filter(|node| node.kind == "Rule")
        .any(|rule| {
            let compact = compact_semantic_text(&rule.name);
            compact.contains("assigneerole")
                && (compact.contains("supportagent")
                    || compact.contains("supportmanager")
                    || compact.contains("supportstaff"))
        })
}

fn check_application_overdue_time_requirements(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "writes") {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" {
            continue;
        }
        let Some(target) = node_by_id.get(&edge.target) else {
            continue;
        };
        if !is_status_write_to(edge, target, "overdue") {
            continue;
        }
        if !action_is_scheduler_trigger(action) {
            continue;
        }
        if action_has_current_time_due_requirement(core, &node_by_id, action) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-APP-002",
                format!(
                    "action {} writes {} to Overdue without a current-time requirement",
                    action.name, target.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &action.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add a requirement such as 'the current time to be later than due_at' to action {}.",
                action.name
            )),
        );
    }
    diagnostics
}

fn action_has_current_time_due_requirement(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> bool {
    outgoing_nodes(core, node_by_id, action, "requires")
        .into_iter()
        .filter(|node| node.kind == "Rule")
        .any(|rule| {
            let compact = compact_semantic_text(&rule.name);
            compact.contains("currenttime")
                && (compact.contains("laterthan") || compact.contains("after"))
                && (compact.contains("dueat") || compact.contains("due"))
        })
}

fn action_is_scheduler_trigger(action: &Node) -> bool {
    action
        .attributes
        .get("trigger")
        .is_some_and(|trigger| trigger.to_ascii_lowercase().contains("scheduler"))
}

fn check_application_status_public_update(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" || !application_has_public_update_surface(core) {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut reported_actions = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "writes") {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" {
            continue;
        }
        let Some(target) = node_by_id.get(&edge.target) else {
            continue;
        };
        if !is_public_ticket_status_transition(edge, target) {
            continue;
        }
        if !reported_actions.insert(action.id.clone()) {
            continue;
        }
        if action_writes_public_update(core, &node_by_id, action) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-APP-003",
                format!(
                    "action {} changes {} without recording a public update",
                    action.name, target.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &action.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add a 'the system records a public update' bullet to action {}.",
                action.name
            )),
        );
    }
    diagnostics
}

fn application_has_public_update_surface(core: &AilCore) -> bool {
    core.graph.nodes.iter().any(|node| {
        let lower = node.name.to_ascii_lowercase();
        matches!(node.kind.as_str(), "Field" | "View")
            && (lower.contains("public update") || lower.contains("public_updates"))
    })
}

fn is_public_ticket_status_transition(edge: &Edge, target: &Node) -> bool {
    if target.kind != "Field" || !target.name.eq_ignore_ascii_case("ticket.status") {
        return false;
    }
    ["new", "assigned", "closed", "overdue"]
        .iter()
        .any(|value| is_status_write_to(edge, target, value))
}

fn is_status_write_to(edge: &Edge, target: &Node, value: &str) -> bool {
    if target.kind != "Field" || !target.name.to_ascii_lowercase().ends_with(".status") {
        return false;
    }
    let Some(provenance) = edge.attributes.get("provenance") else {
        return false;
    };
    let compact = compact_semantic_text(provenance);
    let value = compact_semantic_text(value);
    compact.contains(&format!("statusto{value}")) || compact.contains(&format!("status{value}"))
}

fn action_writes_public_update(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> bool {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "writes" && edge.source == action.id)
        .any(|edge| {
            let target_text = node_by_id
                .get(&edge.target)
                .map(|target| target.name.as_str())
                .unwrap_or("");
            [
                target_text,
                edge.attributes.get("provenance").map_or("", String::as_str),
            ]
            .iter()
            .any(|text| {
                let lower = text.to_ascii_lowercase();
                lower.contains("public update") || lower.contains("public_updates")
            })
        })
}

fn check_application_notification_audit_requirements(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut reported_actions = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "writes") {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" || !action.name.to_ascii_lowercase().contains("notify") {
            continue;
        }
        if !edge_mentions_notification_audit(edge, &node_by_id) {
            continue;
        }
        if !reported_actions.insert(action.id.clone()) {
            continue;
        }
        if action_has_responder_pager_requirement(core, &node_by_id, action) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-APP-004",
                format!(
                    "action {} records a notification audit entry without requiring responder pager",
                    action.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &action.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add a requirement such as 'the system requires responder pager' to action {}.",
                action.name
            )),
        );
    }
    diagnostics
}

fn edge_mentions_notification_audit(edge: &Edge, node_by_id: &BTreeMap<String, Node>) -> bool {
    let target_text = node_by_id
        .get(&edge.target)
        .map(|target| target.name.as_str())
        .unwrap_or("");
    [
        target_text,
        edge.attributes.get("provenance").map_or("", String::as_str),
    ]
    .iter()
    .any(|text| {
        let compact = compact_semantic_text(text);
        compact.contains("notificationauditentry")
    })
}

fn action_has_responder_pager_requirement(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> bool {
    outgoing_nodes(core, node_by_id, action, "requires")
        .into_iter()
        .filter(|node| node.kind == "Rule")
        .any(|rule| compact_semantic_text(&rule.name).contains("responderpager"))
}

fn check_application_incident_lifecycle_status_requirements(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut reported_actions = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "writes") {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" {
            continue;
        }
        for (written_status, required_status) in
            [("Resolved", "Mitigating"), ("Postmortem", "Resolved")]
        {
            if !edge_writes_incident_status_to(edge, written_status) {
                continue;
            }
            let report_key = format!("{}:{written_status}", action.id);
            if !reported_actions.insert(report_key) {
                continue;
            }
            if action_requires_incident_status(core, &node_by_id, action, required_status) {
                continue;
            }
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-APP-005",
                    format!(
                        "action {} writes incident status to {written_status} without requiring incident status {required_status}",
                        action.name
                    ),
                )
                .with_source_provenance(
                    edge.attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &action.id)),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id))
                .with_repair_suggestion(format!(
                    "Add a requirement such as 'the incident status to be {required_status}' to action {}.",
                    action.name
                )),
            );
        }
    }
    diagnostics
}

fn edge_writes_incident_status_to(edge: &Edge, value: &str) -> bool {
    let Some(provenance) = edge.attributes.get("provenance") else {
        return false;
    };
    let compact = compact_semantic_text(provenance);
    let value = compact_semantic_text(value);
    compact.contains(&format!("incidentstatusto{value}"))
        || compact.contains(&format!("incidentstatus{value}"))
}

fn action_requires_incident_status(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
    status: &str,
) -> bool {
    let status = compact_semantic_text(status);
    outgoing_nodes(core, node_by_id, action, "requires")
        .into_iter()
        .filter(|node| node.kind == "Rule")
        .any(|rule| {
            let compact = compact_semantic_text(&rule.name);
            compact.contains("incidentstatus") && compact.contains(&status)
        })
}

fn check_application_private_notes_public_timeline(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut reported_actions = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "writes") {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" {
            continue;
        }
        if !edge_mentions_private_notes_public_timeline(edge, &node_by_id) {
            continue;
        }
        if !reported_actions.insert(action.id.clone()) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-APP-006",
                format!(
                    "action {} writes private notes to the public timeline",
                    action.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &action.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Remove the private-note public timeline write or replace it with a non-secret public summary in action {}.",
                action.name
            )),
        );
    }
    diagnostics
}

fn edge_mentions_private_notes_public_timeline(
    edge: &Edge,
    node_by_id: &BTreeMap<String, Node>,
) -> bool {
    let target_text = node_by_id
        .get(&edge.target)
        .map(|target| target.name.as_str())
        .unwrap_or("");
    [
        target_text,
        edge.attributes.get("provenance").map_or("", String::as_str),
    ]
    .iter()
    .any(|text| {
        let compact = compact_semantic_text(text);
        compact.contains("privatenotes") && compact.contains("publictimeline")
    })
}

fn check_application_incident_escalation_policy_review(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut reported_actions = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "writes") {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" || !action.name.to_ascii_lowercase().contains("escalate") {
            continue;
        }
        if !edge_writes_incident_status_to(edge, "Mitigating") {
            continue;
        }
        if !reported_actions.insert(action.id.clone()) {
            continue;
        }
        if action_requires_commander_review(core, &node_by_id, action) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-APP-007",
                format!(
                    "action {} escalates an incident without requiring commander review",
                    action.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &action.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add a requirement such as 'the escalation policy to require commander review' to action {}.",
                action.name
            )),
        );
    }
    diagnostics
}

fn action_requires_commander_review(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> bool {
    outgoing_nodes(core, node_by_id, action, "requires")
        .into_iter()
        .filter(|node| node.kind == "Rule")
        .any(|rule| {
            let compact = compact_semantic_text(&rule.name);
            compact.contains("commanderreview") || compact.contains("commanderapproval")
        })
}

fn check_application_stateful_counter_runtime_policy(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for action in core.graph.nodes.iter().filter(|node| node.kind == "Action") {
        let Some(write_edge) = action_counter_state_write_edge(core, &node_by_id, action) else {
            continue;
        };
        let action_text = compact_action_runtime_policy_text(core, &node_by_id, action);
        let guarantee_text = compact_action_guarantee_text(core, &node_by_id, action);
        if stateful_text_contains_any(&action_text, &["persist", "durable"])
            && !stateful_text_contains_any(
                &guarantee_text,
                &[
                    "persist",
                    "durable",
                    "journal",
                    "writeaheadlog",
                    "snapshot",
                    "nextreplay",
                ],
            )
        {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-STATE-001",
                    format!(
                        "action {} mutates persistent counter state without a persistence guarantee",
                        action.name
                    ),
                )
                .with_source_provenance(
                    write_edge
                        .attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &action.id)),
                )
                .with_affected_graph_item(format!("edge:{}", write_edge.id))
                .with_repair_suggestion(format!(
                    "Add a guarantee that action {} persists counter state before replay, or remove the persistent-state claim.",
                    action.name
                )),
            );
        }
        if stateful_text_contains_any(&action_text, &["retry", "retryable"])
            && !stateful_text_contains_any(
                &action_text,
                &[
                    "idempotencykey",
                    "requestid",
                    "dedupe",
                    "deduplication",
                    "processedrequest",
                    "duplicate",
                ],
            )
        {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-STATE-002",
                    format!(
                        "action {} is retryable but mutates counter state without an idempotency key",
                        action.name
                    ),
                )
                .with_source_provenance(
                    write_edge
                        .attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &action.id)),
                )
                .with_affected_graph_item(format!("edge:{}", write_edge.id))
                .with_repair_suggestion(format!(
                    "Add a request id or idempotency key requirement and a processed-request write to action {}.",
                    action.name
                )),
            );
        }
        if stateful_text_contains_any(&action_text, &["shared", "concurrent", "parallel"])
            && !action_has_stateful_lock_or_serialization(core, &node_by_id, action)
        {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-STATE-003",
                    format!(
                        "action {} mutates shared counter state without a lock or serialization rule",
                        action.name
                    ),
                )
                .with_source_provenance(
                    write_edge
                        .attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &action.id)),
                )
                .with_affected_graph_item(format!("edge:{}", write_edge.id))
                .with_repair_suggestion(format!(
                    "Add a counter lock requirement, serialization guarantee, or System lock guard for action {}.",
                    action.name
                )),
            );
        }
        if action_has_failure_after_counter_write(core, &node_by_id, action)
            && !stateful_text_contains_any(
                &guarantee_text,
                &[
                    "rollback",
                    "resume",
                    "preserveprior",
                    "preservesprior",
                    "idempotencykey",
                    "requestid",
                ],
            )
        {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-STATE-004",
                    format!(
                        "action {} can fail after a counter write without a replay recovery policy",
                        action.name
                    ),
                )
                .with_source_provenance(
                    write_edge
                        .attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &action.id)),
                )
                .with_affected_graph_item(format!("edge:{}", write_edge.id))
                .with_repair_suggestion(format!(
                    "Add a rollback, resume, or idempotent replay guarantee to action {}.",
                    action.name
                )),
            );
        }
    }
    diagnostics
}

fn check_application_repeated_scheduler_temporal_policy(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for action in core.graph.nodes.iter().filter(|node| node.kind == "Action") {
        if outgoing_edges(core, action, "repeats").is_empty() {
            continue;
        }
        let guarantees = outgoing_nodes(core, &node_by_id, action, "guarantees")
            .into_iter()
            .filter(|node| node.kind == "Guarantee")
            .collect::<Vec<_>>();
        if guarantees.iter().any(is_temporal_policy_guarantee) {
            continue;
        }
        for guarantee in &guarantees {
            if !is_scheduler_claim(guarantee) {
                continue;
            }
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-WORKFLOW-001",
                    format!(
                        "action {} claims scheduler behavior without a temporal policy",
                        action.name
                    ),
                )
                .with_source_provenance(node_provenance(core, &guarantee.id))
                .with_affected_graph_item(format!("node:{}", guarantee.id))
                .with_repair_suggestion(format!(
                    "Add a temporal policy guarantee to action {} or remove the scheduler behavior claim.",
                    action.name
                )),
            );
        }
    }
    diagnostics
}

fn check_application_repeated_scheduler_retry_backoff_policy(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "Application" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for action in core.graph.nodes.iter().filter(|node| node.kind == "Action") {
        if outgoing_edges(core, action, "repeats").is_empty() {
            continue;
        }
        let guarantees = outgoing_nodes(core, &node_by_id, action, "guarantees")
            .into_iter()
            .filter(|node| node.kind == "Guarantee")
            .collect::<Vec<_>>();
        if !guarantees.iter().any(is_scheduler_claim) {
            continue;
        }
        if guarantees.iter().any(is_backoff_policy_guarantee) {
            continue;
        }
        for guarantee in &guarantees {
            if !is_retry_policy_guarantee(guarantee) {
                continue;
            }
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-WORKFLOW-002",
                    format!(
                        "action {} declares retry policy without backoff policy",
                        action.name
                    ),
                )
                .with_source_provenance(node_provenance(core, &guarantee.id))
                .with_affected_graph_item(format!("node:{}", guarantee.id))
                .with_repair_suggestion(format!(
                    "Add a backoff policy guarantee to action {} or remove the retry policy.",
                    action.name
                )),
            );
        }
    }
    diagnostics
}

fn is_scheduler_claim(node: &Node) -> bool {
    let text = compact_semantic_text(&node.name);
    text.contains("schedulerbehavior")
        || text.contains("scheduledbehavior")
        || text.contains("schedulerclaim")
        || text.contains("scheduledtask")
}

fn is_temporal_policy_guarantee(node: &Node) -> bool {
    let text = compact_semantic_text(&node.name);
    text.contains("temporalpolicy")
        || text.contains("schedulepolicy")
        || text.contains("schedulerpolicy")
}

fn is_retry_policy_guarantee(node: &Node) -> bool {
    let text = compact_semantic_text(&node.name);
    text.contains("retrypolicy") || text.contains("retrybudget")
}

fn is_backoff_policy_guarantee(node: &Node) -> bool {
    let text = compact_semantic_text(&node.name);
    text.contains("backoffpolicy")
        || text.contains("exponentialbackoff")
        || text.contains("linearbackoff")
}

fn action_counter_state_write_edge(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> Option<Edge> {
    outgoing_edges(core, action, "writes")
        .into_iter()
        .find(|edge| {
            let Some(target) = node_by_id.get(&edge.target) else {
                return false;
            };
            if target.kind != "Field"
                || !target.name.eq_ignore_ascii_case("Counter.value")
                || target.type_name.as_deref() != Some("Int")
            {
                return false;
            }
            edge.attributes.get("provenance").is_some_and(|provenance| {
                let compact = compact_semantic_text(provenance);
                stateful_text_contains_any(&compact, &["increments", "decrements"])
            })
        })
}

fn compact_action_runtime_policy_text(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> String {
    let mut text = String::new();
    text.push_str(&compact_semantic_text(&action.name));
    for key in ["label", "trigger"] {
        if let Some(value) = action.attributes.get(key) {
            text.push_str(&compact_semantic_text(value));
        }
    }
    for edge in outgoing_edges(core, action, "writes") {
        if let Some(provenance) = edge.attributes.get("provenance") {
            text.push_str(&compact_semantic_text(provenance));
        }
        if let Some(target) = node_by_id.get(&edge.target) {
            text.push_str(&compact_semantic_text(&target.name));
        }
    }
    for edge_kind in ["requires", "reads", "guarantees", "may_fail_with"] {
        for node in outgoing_nodes(core, node_by_id, action, edge_kind) {
            text.push_str(&compact_semantic_text(&node.name));
            if let Some(condition) = node.attributes.get("condition") {
                text.push_str(&compact_semantic_text(condition));
            }
        }
    }
    text
}

fn compact_action_guarantee_text(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> String {
    outgoing_nodes(core, node_by_id, action, "guarantees")
        .into_iter()
        .map(|guarantee| compact_semantic_text(&guarantee.name))
        .collect::<Vec<_>>()
        .join("")
}

fn action_has_stateful_lock_or_serialization(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> bool {
    let action_text = compact_action_runtime_policy_text(core, node_by_id, action);
    if stateful_text_contains_any(
        &action_text,
        &["lock", "serialized", "serialised", "exclusive"],
    ) {
        return true;
    }
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "LockGuard")
        .any(|guard| {
            let mut text = compact_semantic_text(&guard.name);
            for value in guard.attributes.values() {
                text.push_str(&compact_semantic_text(value));
            }
            text.contains("counter") && (text.contains("value") || text.contains("state"))
        })
}

fn action_has_failure_after_counter_write(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
) -> bool {
    outgoing_nodes(core, node_by_id, action, "may_fail_with")
        .into_iter()
        .any(|failure| {
            let mut text = compact_semantic_text(&failure.name);
            if let Some(condition) = failure.attributes.get("condition") {
                text.push_str(&compact_semantic_text(condition));
            }
            text.contains("after") && text.contains("countervalue") && text.contains("written")
        })
}

fn stateful_text_contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

fn check_toolchain_agent_artifact_fingerprint_reads(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.name != "ail-toolchain-agent" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "reads") {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" || !action.name.starts_with("Verify") {
            continue;
        }
        let Some(artifact_field) = node_by_id.get(&edge.target) else {
            continue;
        };
        if artifact_field.kind != "Field"
            || !is_toolchain_agent_artifact_field(&artifact_field.name)
        {
            continue;
        }
        let Some(expected_fingerprint) =
            toolchain_agent_fingerprint_field_name(&artifact_field.name)
        else {
            continue;
        };
        if action_reads_field(core, &node_by_id, action, &expected_fingerprint) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-AGENT-001",
                format!(
                    "action {} verifies {} without reading {}",
                    action.name, artifact_field.name, expected_fingerprint
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &action.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Read {expected_fingerprint} before action {} verifies {}.",
                action.name, artifact_field.name
            )),
        );
    }
    diagnostics
}

fn is_toolchain_agent_artifact_field(field_name: &str) -> bool {
    let lower = field_name.to_ascii_lowercase();
    lower.starts_with("buildrequest.")
        && (lower.ends_with(" artifact") || lower.ends_with(" report"))
        && !lower.ends_with(" fingerprint")
        && !lower.ends_with(" verification report")
        && !lower.ends_with(" compilation report")
        && !lower.ends_with(" review report")
}

fn toolchain_agent_fingerprint_field_name(field_name: &str) -> Option<String> {
    let (prefix, local) = field_name.split_once('.')?;
    if local == "bytecode artifact" {
        Some(format!("{prefix}.bytecode fingerprint"))
    } else if local == "compiler pass artifact" {
        Some(format!("{prefix}.compiler pass fingerprint"))
    } else if local.ends_with(" artifact") || local.ends_with(" report") {
        Some(format!("{field_name} fingerprint"))
    } else {
        None
    }
}

fn action_reads_field(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    action: &Node,
    field_name: &str,
) -> bool {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "reads" && edge.source == action.id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .any(|field| field.kind == "Field" && field.name.eq_ignore_ascii_case(field_name))
}

fn compact_semantic_text(text: &str) -> String {
    text.to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
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
                "AIL-FAILURE-001",
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
                "AIL-TRACE-002",
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

fn check_recursive_function_termination(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for function in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Function")
    {
        let recursive_calls = core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "calls" && edge.source == function.id)
            .filter_map(|edge| node_by_id.get(&edge.target))
            .filter(|call| {
                call.kind == "Call"
                    && call
                        .attributes
                        .get("target")
                        .is_some_and(|target| target == &function.name)
            })
            .collect::<Vec<_>>();
        if recursive_calls.is_empty() {
            continue;
        }
        let has_base_case_branch = core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "contains" && edge.source == function.id)
            .filter_map(|edge| node_by_id.get(&edge.target))
            .any(|node| node.kind == "Branch" && branch_looks_like_base_case(node));
        let has_return = core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "contains" && edge.source == function.id)
            .filter_map(|edge| node_by_id.get(&edge.target))
            .any(|node| node.kind == "Return");
        let has_decreasing_call = recursive_calls
            .iter()
            .any(|call| recursive_call_looks_decreasing(&call.name));
        let has_explicit_termination_bound = core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "has_termination_bound" && edge.source == function.id)
            .filter_map(|edge| node_by_id.get(&edge.target))
            .any(|node| {
                node.kind == "TerminationBound"
                    && node
                        .attributes
                        .get("value")
                        .is_some_and(|value| termination_bound_looks_explicit(value))
            });
        let has_well_founded_measure = core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "has_termination_measure" && edge.source == function.id)
            .filter_map(|edge| node_by_id.get(&edge.target))
            .any(|node| {
                node.kind == "TerminationMeasure"
                    && node
                        .attributes
                        .get("value")
                        .is_some_and(|value| termination_measure_looks_well_founded(value))
            });
        if has_return
            && ((has_base_case_branch && has_decreasing_call)
                || has_explicit_termination_bound
                || has_well_founded_measure)
        {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-CONTROL-003",
                format!(
                    "function {} has unproven recursive termination",
                    function.name
                ),
            )
            .with_source_provenance(node_provenance(core, &function.id))
            .with_affected_graph_item(format!("node:{}", function.id))
            .with_repair_suggestion(format!(
                "Add a base-case branch return, a decreasing recursive argument, an explicit stack/termination bound, or a well-founded termination measure for function {}.",
                function.name
            )),
        );
    }
    diagnostics
}

fn branch_looks_like_base_case(branch: &crate::core_model::Node) -> bool {
    let condition = branch
        .attributes
        .get("condition")
        .unwrap_or(&branch.name)
        .to_ascii_lowercase();
    condition.contains(" is 0")
        || condition.contains(" equals 0")
        || condition.contains(" == 0")
        || condition.contains(" <= 0")
        || condition.contains(" zero")
        || condition.contains(" empty")
        || condition.contains(" none")
        || condition.contains(" null")
        || condition.contains(" base")
}

fn recursive_call_looks_decreasing(call_text: &str) -> bool {
    let call_text = call_text.to_ascii_lowercase();
    call_text.contains(" minus ")
        || call_text.contains(" - ")
        || call_text.contains("decrement")
        || call_text.contains("predecessor")
        || call_text.contains("smaller")
        || call_text.contains("less")
}

fn termination_bound_looks_explicit(bound_text: &str) -> bool {
    let bound_text = bound_text.to_ascii_lowercase();
    (bound_text.contains("recursion")
        || bound_text.contains("stack")
        || bound_text.contains("termination"))
        && bound_text.chars().any(|ch| ch.is_ascii_digit())
}

fn termination_measure_looks_well_founded(measure_text: &str) -> bool {
    let measure_text = measure_text.to_ascii_lowercase();
    (measure_text.contains("termination measure") || measure_text.contains("well-founded measure"))
        && measure_text.contains("decreas")
        && (measure_text.contains(" to 0")
            || measure_text.contains(" toward 0")
            || measure_text.contains("lower bound")
            || measure_text.contains("bounded below")
            || measure_text.contains("well-founded"))
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
                            "Action"
                                | "Dashboard"
                                | "ExternalBinding"
                                | "Failure"
                                | "Form"
                                | "Function"
                                | "Route"
                                | "Workflow"
                                | "Tool"
                                | "SystemComponent"
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
                (edge.kind == "requires" || edge.kind == "validates")
                    && edge.target == rule.id
                    && node_by_id.get(&edge.source).is_some_and(|source| {
                        matches!(source.kind.as_str(), "Action" | "Form" | "Tool")
                    })
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

fn check_imported_action_effect_grants(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut granted_effects = BTreeMap::<String, BTreeSet<String>>::new();
    for grant in &core.package.capability_grants {
        granted_effects
            .entry(grant.package.clone())
            .or_default()
            .extend(grant.effects.iter().cloned());
    }

    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| {
        matches!(
            edge.kind.as_str(),
            "reads" | "writes" | "calls" | "protects_secret" | "repeats"
        )
    }) {
        let Some(action) = node_by_id.get(&edge.source) else {
            continue;
        };
        if action.kind != "Action" {
            continue;
        }
        let Some(effect) = node_by_id.get(&edge.target) else {
            continue;
        };
        if effect.kind != "Effect" {
            continue;
        }
        let Some(import) = core.package.imports.iter().find(|import| {
            action
                .name
                .strip_prefix(&format!("{}.", import.alias))
                .is_some()
        }) else {
            continue;
        };
        let is_granted = [
            Some(import.alias.as_str()),
            Some(import.path.as_str()),
            import.resolved_package.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|package| {
            granted_effects
                .get(package)
                .is_some_and(|effects| effects.contains(&effect.name))
        });
        if is_granted {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-PACKAGE-001",
                format!(
                    "imported action {} uses effect '{}' without a capability grant for import {}",
                    action.name, effect.name, import.alias
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
                "Add a capability-grants entry for package {} with effect '{}'.",
                import.alias, effect.name
            )),
        );
    }
    diagnostics
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
                "AIL-TRACE-001",
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

fn check_external_binding_trace_coverage(core: &AilCore) -> Vec<AilDiagnostic> {
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "ExternalBinding")
        .filter(|binding| !has_outgoing_edge(&core.graph, "records_trace", &binding.id))
        .map(|binding| {
            AilDiagnostic::error(
                "AIL-TRACE-001",
                format!(
                    "external binding {} is missing foreign-call trace coverage",
                    binding.name
                ),
            )
            .with_source_provenance(node_provenance(core, &binding.id))
            .with_affected_graph_item(format!("node:{}", binding.id))
            .with_repair_suggestion(format!(
                "Add a trace event to external binding {}.",
                binding.name
            ))
        })
        .collect()
}

fn check_external_binding_status_maps(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "ExternalBinding")
        .filter(|binding| {
            let has_status_output = core.graph.edges.iter().any(|edge| {
                edge.kind == "has_output"
                    && edge.source == binding.id
                    && node_by_id.get(&edge.target).is_some_and(|output| {
                        output.type_name.as_deref() == Some("CInt")
                            || output.name.rsplit('.').next() == Some("status")
                    })
            });
            let has_status_map = has_outgoing_edge(&core.graph, "maps_status", &binding.id)
                || has_outgoing_edge(&core.graph, "may_fail_with", &binding.id);
            has_status_output && !has_status_map
        })
        .map(|binding| {
            AilDiagnostic::error(
                "AIL-FFI-ERRNO-001",
                format!(
                    "external binding {} returns status without errno or status mapping",
                    binding.name
                ),
            )
            .with_source_provenance(node_provenance(core, &binding.id))
            .with_affected_graph_item(format!("node:{}", binding.id))
            .with_repair_suggestion(format!(
                "Add a '{} maps errno or status codes:' section.",
                binding.name.rsplit('.').next().unwrap_or(&binding.name)
            ))
        })
        .collect()
}

fn check_external_binding_pointer_ownership(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "has_input")
    {
        let Some(binding) = node_by_id.get(&edge.source) else {
            continue;
        };
        if binding.kind != "ExternalBinding" {
            continue;
        }
        let Some(input) = node_by_id.get(&edge.target) else {
            continue;
        };
        let ownership = input
            .attributes
            .get("ownership")
            .map(String::as_str)
            .unwrap_or("");
        if ownership.contains("borrowed")
            && (ownership.contains("escaping")
                || ownership.contains("stores")
                || ownership.contains("after return"))
        {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-FFI-OWNERSHIP-001",
                    format!(
                        "borrowed pointer {} cannot escape the C call boundary",
                        input.name
                    ),
                )
                .with_source_provenance(node_provenance(core, &input.id))
                .with_affected_graph_item(format!("node:{}", input.id))
                .with_repair_suggestion(format!(
                    "Use owned pointer ownership for {} or remove the escape behavior.",
                    input.name
                )),
            );
        }
    }
    diagnostics
}

fn check_external_binding_owned_pointer_release(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| matches!(edge.kind.as_str(), "has_input" | "has_output"))
    {
        let Some(binding) = node_by_id.get(&edge.source) else {
            continue;
        };
        if binding.kind != "ExternalBinding" {
            continue;
        }
        let Some(value) = node_by_id.get(&edge.target) else {
            continue;
        };
        let ownership = value
            .attributes
            .get("ownership")
            .map(String::as_str)
            .unwrap_or("");
        if ownership_contains_token(ownership, "owned") && !ownership.contains("release") {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-FFI-OWNERSHIP-002",
                    format!(
                        "owned pointer {} crosses C boundary without release semantics",
                        value.name
                    ),
                )
                .with_source_provenance(node_provenance(core, &value.id))
                .with_affected_graph_item(format!("node:{}", value.id))
                .with_repair_suggestion(format!(
                    "Add release semantics such as 'owned release free' to {}.",
                    value.name
                )),
            );
        }
    }
    diagnostics
}

fn check_external_binding_nullable_non_null(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| matches!(edge.kind.as_str(), "has_input" | "has_output"))
    {
        let Some(binding) = node_by_id.get(&edge.source) else {
            continue;
        };
        if binding.kind != "ExternalBinding" {
            continue;
        }
        let Some(value) = node_by_id.get(&edge.target) else {
            continue;
        };
        let ownership = value
            .attributes
            .get("ownership")
            .map(String::as_str)
            .unwrap_or("");
        if value
            .type_name
            .as_deref()
            .is_some_and(|type_name| type_name.starts_with("NonNull<"))
            && ownership_contains_token(ownership, "nullable")
        {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-FFI-NULL-001",
                    format!(
                        "nullable value {} cannot satisfy NonNull pointer contract",
                        value.name
                    ),
                )
                .with_source_provenance(node_provenance(core, &value.id))
                .with_affected_graph_item(format!("node:{}", value.id))
                .with_repair_suggestion(format!(
                    "Use Nullable<Pointer<T>> or remove the nullable ownership marker from {}.",
                    value.name
                )),
            );
        }
    }
    diagnostics
}

fn check_external_binding_mutable_aliases(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for binding in core
        .graph
        .nodes
        .iter()
        .filter(|node| node.kind == "ExternalBinding")
    {
        let mut mutable_aliases = BTreeMap::<String, Vec<Node>>::new();
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "has_input" && edge.source == binding.id)
        {
            let Some(input) = node_by_id.get(&edge.target) else {
                continue;
            };
            let ownership = input
                .attributes
                .get("ownership")
                .map(String::as_str)
                .unwrap_or("");
            if ownership.contains("borrowed mutable")
                && let Some(alias) = ownership_alias_group(ownership)
            {
                mutable_aliases
                    .entry(alias.to_string())
                    .or_default()
                    .push(input.clone());
            }
        }
        for (alias, inputs) in mutable_aliases {
            if inputs.len() < 2 {
                continue;
            }
            let names = inputs
                .iter()
                .map(|input| input.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-FFI-ALIAS-001",
                    format!(
                        "external binding {} has aliased mutable pointer group {} across {}",
                        binding.name, alias, names
                    ),
                )
                .with_source_provenance(node_provenance(core, &inputs[0].id))
                .with_affected_graph_item(format!("node:{}", inputs[0].id))
                .with_repair_suggestion(format!(
                    "Split mutable alias group {} or pass only one mutable borrowed pointer.",
                    alias
                )),
            );
        }
    }
    diagnostics
}

fn check_external_binding_secret_leakage(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| matches!(edge.kind.as_str(), "has_input" | "has_output"))
    {
        let Some(binding) = node_by_id.get(&edge.source) else {
            continue;
        };
        if binding.kind != "ExternalBinding" {
            continue;
        }
        let Some(value) = node_by_id.get(&edge.target) else {
            continue;
        };
        let type_name = value.type_name.as_deref().unwrap_or("");
        let ownership = value
            .attributes
            .get("ownership")
            .map(String::as_str)
            .unwrap_or("");
        if type_contains_secret(type_name) && !ownership.contains("redacted") {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-FFI-SECRET-001",
                    format!(
                        "secret value {} crosses foreign boundary without redaction semantics",
                        value.name
                    ),
                )
                .with_source_provenance(node_provenance(core, &value.id))
                .with_affected_graph_item(format!("node:{}", value.id))
                .with_repair_suggestion(format!(
                    "Remove secret type from {} or mark the boundary as redacted.",
                    value.name
                )),
            );
        }
    }
    diagnostics
}

fn ownership_contains_token(ownership: &str, token: &str) -> bool {
    ownership
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .any(|part| part.eq_ignore_ascii_case(token))
}

fn ownership_alias_group(ownership: &str) -> Option<&str> {
    let mut parts = ownership.split_whitespace();
    while let Some(part) = parts.next() {
        if part == "alias" {
            return parts.next();
        }
    }
    None
}

fn check_ui_form_action_targets(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "calls") {
        let Some(form) = node_by_id.get(&edge.source) else {
            continue;
        };
        if form.kind != "Form" {
            continue;
        }
        let Some(action) = node_by_id.get(&edge.target) else {
            continue;
        };
        if node_provenance(core, &action.id).is_none() {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-UI-FORM-001",
                    format!("form {} calls undeclared action {}", form.name, action.name),
                )
                .with_source_provenance(
                    edge.attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &form.id)),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id))
                .with_repair_suggestion(format!(
                    "Declare Action: {}. before the form calls it.",
                    title_from_pascal_case(&action.name)
                )),
            );
        }
    }
    diagnostics
}

fn check_ui_route_permissions(core: &AilCore) -> Vec<AilDiagnostic> {
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Route")
        .filter(|route| has_outgoing_edge(&core.graph, "reads", &route.id))
        .filter(|route| !has_outgoing_edge(&core.graph, "requires", &route.id))
        .map(|route| {
            AilDiagnostic::error(
                "AIL-UI-PERMISSION-002",
                format!(
                    "route {} reads data without a matching permission",
                    route.name
                ),
            )
            .with_source_provenance(node_provenance(core, &route.id))
            .with_affected_graph_item(format!("node:{}", route.id))
            .with_repair_suggestion(format!(
                "Add 'The route requires permission:' to route {}.",
                route.name
            ))
        })
        .collect()
}

fn check_ui_dashboard_permissions(core: &AilCore) -> Vec<AilDiagnostic> {
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Dashboard")
        .filter(|dashboard| has_outgoing_edge(&core.graph, "reads", &dashboard.id))
        .filter(|dashboard| !has_outgoing_edge(&core.graph, "requires", &dashboard.id))
        .map(|dashboard| {
            AilDiagnostic::error(
                "AIL-UI-PERMISSION-001",
                format!(
                    "dashboard {} reads data without a matching permission",
                    dashboard.name
                ),
            )
            .with_source_provenance(node_provenance(core, &dashboard.id))
            .with_affected_graph_item(format!("node:{}", dashboard.id))
            .with_repair_suggestion(format!(
                "Add 'The dashboard requires permission:' to dashboard {}.",
                dashboard.name
            ))
        })
        .collect()
}

fn check_ui_form_accessibility(core: &AilCore) -> Vec<AilDiagnostic> {
    core.graph
        .nodes
        .iter()
        .filter(|node| node.kind == "Form")
        .filter(|form| has_outgoing_edge(&core.graph, "validates", &form.id))
        .filter(|form| has_outgoing_edge(&core.graph, "records_trace", &form.id))
        .filter(|form| !has_outgoing_edge(&core.graph, "has_accessibility", &form.id))
        .map(|form| {
            AilDiagnostic::error(
                "AIL-UI-A11Y-001",
                format!(
                    "form {} records validation failure trace without accessibility announcement",
                    form.name
                ),
            )
            .with_source_provenance(node_provenance(core, &form.id))
            .with_affected_graph_item(format!("node:{}", form.id))
            .with_repair_suggestion(format!(
                "Add 'The form accessibility is:' to form {}.",
                form.name
            ))
        })
        .collect()
}

fn check_ui_destructive_action_confirmations(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "calls") {
        let Some(form) = node_by_id.get(&edge.source) else {
            continue;
        };
        if form.kind != "Form" || has_outgoing_edge(&core.graph, "requires_confirmation", &form.id)
        {
            continue;
        }
        let Some(action) = node_by_id.get(&edge.target) else {
            continue;
        };
        if action.kind != "Action" || !ui_action_is_destructive(core, &node_by_id, &action.id) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-UI-CONFIRM-001",
                format!(
                    "form {} exposes destructive action {} without confirmation",
                    form.name, action.name
                ),
            )
            .with_source_provenance(
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &form.id)),
            )
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add 'The form requires confirmation:' to form {}.",
                form.name
            )),
        );
    }
    diagnostics
}

fn ui_action_is_destructive(
    core: &AilCore,
    node_by_id: &BTreeMap<String, crate::core_model::Node>,
    action_id: &str,
) -> bool {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "writes" && edge.source == action_id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .any(|target| ui_write_is_destructive(&target.name))
}

fn ui_write_is_destructive(write: &str) -> bool {
    let normalized = write.trim().to_ascii_lowercase();
    [
        "delete ", "deletes ", "remove ", "removes ", "close ", "closes ", "cancel ", "cancels ",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
}

fn check_ui_workflow_step_order(core: &AilCore) -> Vec<AilDiagnostic> {
    let node_by_id = graph_node_by_id(core);
    let mut steps_by_workflow = BTreeMap::<String, Vec<(String, String)>>::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "contains")
    {
        let Some(workflow) = node_by_id.get(&edge.source) else {
            continue;
        };
        let Some(step) = node_by_id.get(&edge.target) else {
            continue;
        };
        if workflow.kind != "Workflow" || step.kind != "Step" {
            continue;
        }
        let label = step
            .attributes
            .get("label")
            .cloned()
            .unwrap_or_else(|| step.name.clone());
        steps_by_workflow
            .entry(workflow.name.clone())
            .or_default()
            .push((step.id.clone(), label));
    }

    let mut diagnostics = Vec::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "blocks_before")
    {
        let Some(blocked_step) = node_by_id.get(&edge.source) else {
            continue;
        };
        let Some(prerequisite_step) = node_by_id.get(&edge.target) else {
            continue;
        };
        let Some((workflow_name, _)) = blocked_step.name.split_once('.') else {
            continue;
        };
        let Some(steps) = steps_by_workflow.get(workflow_name) else {
            continue;
        };
        let blocked_index = steps.iter().position(|(id, _)| id == &blocked_step.id);
        let prerequisite_index = steps.iter().position(|(id, _)| id == &prerequisite_step.id);
        if let (Some(blocked_index), Some(prerequisite_index)) = (blocked_index, prerequisite_index)
            && blocked_index <= prerequisite_index
        {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-UI-WORKFLOW-001",
                    format!(
                        "workflow {workflow_name} lists blocked step '{}' before prerequisite '{}'",
                        blocked_step
                            .attributes
                            .get("label")
                            .map(String::as_str)
                            .unwrap_or(&blocked_step.name),
                        prerequisite_step
                            .attributes
                            .get("label")
                            .map(String::as_str)
                            .unwrap_or(&prerequisite_step.name)
                    ),
                )
                .with_source_provenance(
                    edge.attributes
                        .get("provenance")
                        .cloned()
                        .or_else(|| node_provenance(core, &blocked_step.id)),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id))
                .with_repair_suggestion(format!(
                    "Move '{}' after '{}' in workflow {workflow_name}.",
                    blocked_step
                        .attributes
                        .get("label")
                        .map(String::as_str)
                        .unwrap_or(&blocked_step.name),
                    prerequisite_step
                        .attributes
                        .get("label")
                        .map(String::as_str)
                        .unwrap_or(&prerequisite_step.name)
                )),
            );
        }
    }
    diagnostics
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

fn check_tool_provider_call_audit_evidence(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "AgentTool" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "calls") {
        let Some(tool) = node_by_id.get(&edge.source) else {
            continue;
        };
        if tool.kind != "Tool" || tool_has_audit_evidence(core, &node_by_id, tool) {
            continue;
        }
        let Some(call) = node_by_id.get(&edge.target) else {
            continue;
        };
        if !is_external_provider_call(&call.name) {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-AGENT-AUDIT-001",
                format!(
                    "tool {} calls {} without audit evidence",
                    tool.name, call.name
                ),
            )
            .with_source_provenance(node_provenance(core, &call.id).or_else(|| {
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &tool.id))
            }))
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add an audit write or audit-trace guarantee for provider call {} in tool {}.",
                call.name, tool.name
            )),
        );
    }
    diagnostics
}

fn is_external_provider_call(call: &str) -> bool {
    let normalized = call.to_ascii_lowercase();
    normalized.contains("provider.") || normalized.contains("provider ")
}

fn tool_has_audit_evidence(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    tool: &Node,
) -> bool {
    core.graph
        .edges
        .iter()
        .filter(|edge| {
            edge.source == tool.id && matches!(edge.kind.as_str(), "writes" | "guarantees")
        })
        .filter_map(|edge| node_by_id.get(&edge.target))
        .any(|target| {
            let compact = compact_semantic_text(&target.name);
            compact.contains("audit") || compact.contains("ledger")
        })
}

fn check_tool_provider_call_failure_policy(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "AgentTool" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "calls") {
        let Some(tool) = node_by_id.get(&edge.source) else {
            continue;
        };
        if tool.kind != "Tool" {
            continue;
        }
        let Some(call) = node_by_id.get(&edge.target) else {
            continue;
        };
        if !is_external_provider_call(&call.name)
            || !tool_provider_failures(core, &node_by_id, tool, &call.name).is_empty()
        {
            continue;
        }
        diagnostics.push(
            AilDiagnostic::error(
                "AIL-AGENT-FAILURE-001",
                format!(
                    "tool {} calls {} without provider failure policy",
                    tool.name, call.name
                ),
            )
            .with_source_provenance(node_provenance(core, &call.id).or_else(|| {
                edge.attributes
                    .get("provenance")
                    .cloned()
                    .or_else(|| node_provenance(core, &tool.id))
            }))
            .with_affected_graph_item(format!("edge:{}", edge.id))
            .with_repair_suggestion(format!(
                "Add a Failure section for provider call {} in tool {}.",
                call.name, tool.name
            )),
        );
    }
    diagnostics
}

fn check_tool_provider_failure_recovery_policy(core: &AilCore) -> Vec<AilDiagnostic> {
    if core.package.profile != "AgentTool" {
        return Vec::new();
    }
    let node_by_id = graph_node_by_id(core);
    let mut diagnostics = Vec::new();
    for edge in core.graph.edges.iter().filter(|edge| edge.kind == "calls") {
        let Some(tool) = node_by_id.get(&edge.source) else {
            continue;
        };
        if tool.kind != "Tool" {
            continue;
        }
        let Some(call) = node_by_id.get(&edge.target) else {
            continue;
        };
        if !is_external_provider_call(&call.name) {
            continue;
        }
        for failure in tool_provider_failures(core, &node_by_id, tool, &call.name) {
            if failure_has_recovery_policy(core, &node_by_id, failure) {
                continue;
            }
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-AGENT-RECOVERY-001",
                    format!(
                        "failure {} for tool {} has no recovery policy for {}",
                        failure.name, tool.name, call.name
                    ),
                )
                .with_source_provenance(node_provenance(core, &failure.id))
                .with_affected_graph_item(format!("node:{}", failure.id))
                .with_repair_suggestion(format!(
                    "Add retry, fallback, queue, escalation, or human-review handling to Failure {}.",
                    failure.name
                )),
            );
        }
    }
    diagnostics
}

fn tool_provider_failures<'a>(
    core: &'a AilCore,
    node_by_id: &'a BTreeMap<String, Node>,
    tool: &Node,
    call_name: &str,
) -> Vec<&'a Node> {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "may_fail_with" && edge.source == tool.id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .filter(|failure| {
            failure.kind == "Failure"
                && failure
                    .attributes
                    .get("declared")
                    .is_some_and(|value| value == "true")
                && failure_matches_provider_call(failure, call_name)
        })
        .collect()
}

fn failure_matches_provider_call(failure: &Node, call_name: &str) -> bool {
    let compact_failure = compact_semantic_text(&format!(
        "{} {}",
        failure.name,
        failure
            .attributes
            .get("condition")
            .map(String::as_str)
            .unwrap_or("")
    ));
    let compact_call = compact_semantic_text(call_name);
    let compact_provider = compact_semantic_text(
        call_name
            .split_once('.')
            .map_or(call_name, |(provider, _)| provider),
    );
    compact_failure.contains(&compact_call)
        || (!compact_provider.is_empty() && compact_failure.contains(&compact_provider))
        || compact_failure.contains("provider")
}

fn failure_has_recovery_policy(
    core: &AilCore,
    node_by_id: &BTreeMap<String, Node>,
    failure: &Node,
) -> bool {
    core.graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "handles_failure" && edge.source == failure.id)
        .filter_map(|edge| node_by_id.get(&edge.target))
        .any(|handling| is_recovery_policy_text(&handling.name))
}

fn is_recovery_policy_text(text: &str) -> bool {
    let compact = compact_semantic_text(text);
    [
        "retry",
        "retri",
        "backoff",
        "fallback",
        "queue",
        "escalat",
        "humanreview",
        "manualreview",
        "reviewtask",
    ]
    .iter()
    .any(|term| compact.contains(term))
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
        .or_else(|| rule.split_once(" not "))
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
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "CInt"
            | "CChar"
            | "Void"
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
    if type_name.len() == 1
        && type_name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return true;
    }
    if let Some(values) = generic_inner(type_name, "State") {
        return values
            .split(',')
            .map(str::trim)
            .all(|value| !value.is_empty());
    }
    if type_name.starts_with("Callback<") && type_name.ends_with('>') {
        return true;
    }
    for wrapper in ["Option", "List", "Secret", "Pointer", "Nullable", "NonNull"] {
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

fn parse_markdown_thing_heading(line: &str) -> Option<String> {
    if line.contains(' ') || line.ends_with(':') {
        return None;
    }
    let first = line.chars().next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    match line {
        "Action" | "Actions" | "Application" | "Data" | "Entities" | "Failures" | "Profile"
        | "Records" | "Traces" | "Users" | "Views" => None,
        _ => Some(line.to_string()),
    }
}

fn parse_action_header(line: &str) -> Option<String> {
    if let Some(label) = line.strip_prefix("Action: ") {
        return Some(label.trim().trim_end_matches('.').to_string());
    }
    if line.ends_with('.')
        && line.contains(' ')
        && !line.contains(':')
        && line
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return Some(line.trim_end_matches('.').to_string());
    }
    None
}

fn parse_tool_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Tool: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_compiler_pass_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Compiler pass: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_function_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Function: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_type_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Type: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_type_variants_header(document: &AilDocument, line: &str) -> Option<String> {
    let subject = line.strip_suffix(" has variants:")?;
    document
        .types
        .values()
        .find(|type_decl| type_base_name(&type_decl.name) == subject.trim())
        .map(|type_decl| type_decl.name.clone())
}

fn type_base_name(type_name: &str) -> &str {
    type_name
        .split_once('<')
        .map_or(type_name, |(base, _)| base)
}

fn parse_route_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Route: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_route_section(line: &str) -> Option<RouteSection> {
    match line {
        "The route path is:" => Some(RouteSection::Path),
        "The route reads:" => Some(RouteSection::Reads),
        "The route requires permission:" => Some(RouteSection::Permissions),
        "The route records trace:" => Some(RouteSection::Traces),
        _ => None,
    }
}

fn parse_form_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Form: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_form_section(line: &str) -> Option<FormSection> {
    match line {
        "The form calls action:" => Some(FormSection::Action),
        "The form fields are:" => Some(FormSection::Fields),
        "The form validates:" => Some(FormSection::Validations),
        "If form validation fails:" => Some(FormSection::FailureTraces),
        "The form requires confirmation:" => Some(FormSection::Confirmations),
        "The form accessibility is:" => Some(FormSection::Accessibility),
        _ => None,
    }
}

fn parse_dashboard_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Dashboard: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_dashboard_section(line: &str) -> Option<DashboardSection> {
    match line {
        "The dashboard reads:" => Some(DashboardSection::Reads),
        "The dashboard requires permission:" => Some(DashboardSection::Permissions),
        "The dashboard filters:" => Some(DashboardSection::Filters),
        "The dashboard records trace:" => Some(DashboardSection::Traces),
        _ => None,
    }
}

fn parse_workflow_header(line: &str) -> Option<String> {
    let label = line.strip_prefix("Workflow: ")?;
    Some(label.trim().trim_end_matches('.').to_string())
}

fn parse_workflow_section(line: &str) -> Option<WorkflowSection> {
    match line {
        "The workflow steps are:" => Some(WorkflowSection::Steps),
        "The workflow blocks:" => Some(WorkflowSection::Blocks),
        "The workflow records trace:" => Some(WorkflowSection::Traces),
        _ => None,
    }
}

fn parse_function_section(line: &str) -> Option<FunctionSection> {
    match line {
        "The function needs:" => Some(FunctionSection::Inputs),
        "The function produces:" => Some(FunctionSection::Outputs),
        _ => None,
    }
}

fn parse_function_body_header(line: &str) -> Option<String> {
    let name = line.strip_prefix("When ")?;
    let name = name.strip_suffix(" runs:")?;
    Some(name.trim().to_string())
}

fn parse_c_library_header(line: &str) -> Option<String> {
    let library = line.strip_prefix("C library: ")?;
    Some(library.trim().trim_end_matches('.').to_string())
}

fn parse_external_function_import(
    line: &str,
    current_library: Option<&str>,
) -> Option<(String, String)> {
    if let Some(symbol) = line
        .strip_prefix("The library imports function ")
        .and_then(|symbol| symbol.strip_suffix('.'))
    {
        let library = current_library?;
        return Some((library.to_string(), symbol.trim().to_string()));
    }
    let rest = line.strip_prefix("Import function ")?;
    let rest = rest.strip_suffix('.')?;
    let (symbol, library) = rest.split_once(" from ")?;
    Some((library.trim().to_string(), symbol.trim().to_string()))
}

fn parse_external_binding_section(
    document: &AilDocument,
    line: &str,
) -> Option<(String, ExternalBindingSection)> {
    let section_specs = [
        (" needs:", ExternalBindingSection::Inputs),
        (" produces:", ExternalBindingSection::Outputs),
        (
            " maps errno or status codes:",
            ExternalBindingSection::StatusMaps,
        ),
        (
            " requires capability:",
            ExternalBindingSection::Capabilities,
        ),
        (" records trace:", ExternalBindingSection::Traces),
    ];
    for (suffix, section) in section_specs {
        let Some(subject) = line.strip_suffix(suffix) else {
            continue;
        };
        let binding_name = external_binding_name_for_subject(document, subject.trim())?;
        return Some((binding_name, section));
    }
    None
}

fn parse_external_trace_event_line(document: &AilDocument, line: &str) -> Option<(String, String)> {
    let (subject, trace) = line.split_once(" records trace event named ")?;
    let binding_name = external_binding_name_for_subject(document, subject.trim())?;
    Some((binding_name, trim_sentence(trace)))
}

fn external_binding_name_for_subject(document: &AilDocument, subject: &str) -> Option<String> {
    document
        .external_bindings
        .values()
        .find(|binding| binding.symbol == subject || binding.name == subject)
        .map(|binding| binding.name.clone())
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
    let (name, type_name) = parse_typed_bullet(bullet, line_number)?;
    let type_name = normalize_type_name(&type_name);
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

fn parse_compact_thing_bullet(document: &mut AilDocument, bullet: &str) -> Option<()> {
    let (thing_name, fields) = bullet.split_once(" has ")?;
    let thing_name = thing_name.trim();
    if thing_name.is_empty()
        || thing_name.contains(' ')
        || !thing_name.chars().next()?.is_ascii_uppercase()
    {
        return None;
    }
    let provenance = format!("thing:{thing_name}");
    document
        .things
        .entry(thing_name.to_string())
        .or_insert_with(|| AilThing {
            name: thing_name.to_string(),
            fields: BTreeMap::new(),
            provenance,
        });
    for field in split_top_level_commas(fields.trim_end_matches('.')) {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, type_name) = compact_field_parts(field)?;
        let type_name = normalize_type_name(&type_name);
        let is_secret = type_contains_secret(&type_name);
        if let Some(thing) = document.things.get_mut(thing_name) {
            thing.fields.insert(
                name.clone(),
                AilField {
                    name: name.clone(),
                    type_name,
                    is_secret,
                    provenance: format!("field:{thing_name}.{name}"),
                },
            );
        }
    }
    Some(())
}

fn split_top_level_commas(text: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                fields.push(&text[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    fields.push(&text[start..]);
    fields
}

fn compact_field_parts(field: &str) -> Option<(String, String)> {
    if let Some((name, type_name)) = field.split_once(':') {
        return Some((name.trim().to_string(), type_name.trim().to_string()));
    }
    let (name, type_name) = field.split_once(' ')?;
    Some((name.trim().to_string(), type_name.trim().to_string()))
}

fn parse_typed_bullet(bullet: &str, line_number: usize) -> Result<(String, String), String> {
    let Some((name, type_name)) = bullet.split_once(':') else {
        return Err(format!("line {line_number}: expected '<name>: <type>'"));
    };
    Ok((name.trim().to_string(), type_name.trim().to_string()))
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

fn parse_function_bullet(
    document: &mut AilDocument,
    function_name: &str,
    section: FunctionSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let function = document
        .functions
        .get_mut(function_name)
        .ok_or_else(|| format!("line {line_number}: unknown function {function_name}"))?;
    match section {
        FunctionSection::Inputs => {
            let (name, type_name) = parse_typed_bullet(bullet, line_number)?;
            function.inputs.insert(
                name.clone(),
                AilFunctionValue {
                    name: name.clone(),
                    type_name: normalize_type_name(&type_name),
                    provenance: format!("function:{function_name}.input:{name}"),
                },
            );
        }
        FunctionSection::Outputs => {
            let (name, type_name) = parse_typed_bullet(bullet, line_number)?;
            function.outputs.insert(
                name.clone(),
                AilFunctionValue {
                    name: name.clone(),
                    type_name: normalize_type_name(&type_name),
                    provenance: format!("function:{function_name}.output:{name}"),
                },
            );
        }
        FunctionSection::Body => parse_function_body_bullet(function, bullet),
    }
    Ok(())
}

fn parse_type_bullet(
    document: &mut AilDocument,
    type_name: &str,
    section: TypeSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let type_decl = document
        .types
        .get_mut(type_name)
        .ok_or_else(|| format!("line {line_number}: unknown type {type_name}"))?;
    match section {
        TypeSection::Variants => {
            let variant = parse_variant_bullet(type_name, bullet, line_number)?;
            type_decl.variants.insert(variant.name.clone(), variant);
        }
    }
    Ok(())
}

fn parse_route_bullet(
    document: &mut AilDocument,
    route_name: &str,
    section: RouteSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let route = document
        .routes
        .get_mut(route_name)
        .ok_or_else(|| format!("line {line_number}: unknown route {route_name}"))?;
    match section {
        RouteSection::Path => route.path = trim_sentence(bullet),
        RouteSection::Reads => route.reads.push(trim_sentence(bullet)),
        RouteSection::Permissions => route.permissions.push(trim_sentence(bullet)),
        RouteSection::Traces => route.traces.push(trim_sentence(bullet)),
    }
    Ok(())
}

fn parse_form_bullet(
    document: &mut AilDocument,
    form_name: &str,
    section: FormSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let form = document
        .forms
        .get_mut(form_name)
        .ok_or_else(|| format!("line {line_number}: unknown form {form_name}"))?;
    match section {
        FormSection::Action => form.action = Some(action_name_from_label(&trim_sentence(bullet))),
        FormSection::Fields => {
            let (name, type_name) = parse_typed_bullet(bullet, line_number)?;
            form.fields.insert(
                name.clone(),
                AilFormField {
                    name: name.clone(),
                    type_name: normalize_type_name(&type_name),
                    provenance: format!("form:{form_name}.field:{name}"),
                },
            );
        }
        FormSection::Validations => form.validations.push(trim_sentence(bullet)),
        FormSection::FailureTraces => form.failure_traces.push(trim_sentence(bullet)),
        FormSection::Confirmations => form.confirmations.push(trim_sentence(bullet)),
        FormSection::Accessibility => form.accessibility.push(trim_sentence(bullet)),
    }
    Ok(())
}

fn parse_dashboard_bullet(
    document: &mut AilDocument,
    dashboard_name: &str,
    section: DashboardSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let dashboard = document
        .dashboards
        .get_mut(dashboard_name)
        .ok_or_else(|| format!("line {line_number}: unknown dashboard {dashboard_name}"))?;
    match section {
        DashboardSection::Reads => dashboard.reads.push(trim_sentence(bullet)),
        DashboardSection::Permissions => dashboard.permissions.push(trim_sentence(bullet)),
        DashboardSection::Filters => dashboard.filters.push(trim_sentence(bullet)),
        DashboardSection::Traces => dashboard.traces.push(trim_sentence(bullet)),
    }
    Ok(())
}

fn parse_workflow_bullet(
    document: &mut AilDocument,
    workflow_name: &str,
    section: WorkflowSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let workflow = document
        .workflows
        .get_mut(workflow_name)
        .ok_or_else(|| format!("line {line_number}: unknown workflow {workflow_name}"))?;
    match section {
        WorkflowSection::Steps => workflow.steps.push(trim_sentence(bullet)),
        WorkflowSection::Blocks => {
            let text = trim_sentence(bullet);
            let Some((blocked_step, prerequisite_step)) = text.split_once(" before ") else {
                return Err(format!(
                    "line {line_number}: expected '<blocked step> before <prerequisite step>'"
                ));
            };
            workflow.blocks.push(AilWorkflowBlock {
                blocked_step: blocked_step.trim().to_string(),
                prerequisite_step: prerequisite_step.trim().to_string(),
                provenance: format!("workflow:{workflow_name}.block:{text}"),
            });
        }
        WorkflowSection::Traces => workflow.traces.push(trim_sentence(bullet)),
    }
    Ok(())
}

fn parse_variant_bullet(
    type_name: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilVariant, String> {
    let bullet = trim_sentence(bullet);
    let (label, fields) = if let Some((label, payloads)) = bullet.split_once('(') {
        let payloads = payloads
            .strip_suffix(')')
            .ok_or_else(|| format!("line {line_number}: malformed variant payload list"))?;
        let fields = payloads
            .split(',')
            .map(str::trim)
            .filter(|payload| !payload.is_empty())
            .map(|payload| {
                let (name, field_type_name) = parse_typed_bullet(payload, line_number)?;
                Ok((
                    name.clone(),
                    AilVariantField {
                        name: name.clone(),
                        type_name: normalize_type_name(&field_type_name),
                        provenance: format!("type:{type_name}.variant:{label}.field:{name}"),
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        (label.trim().to_string(), fields)
    } else {
        (bullet, BTreeMap::new())
    };
    Ok(AilVariant {
        name: label.clone(),
        label: label.clone(),
        fields,
        provenance: format!("type:{type_name}.variant:{label}"),
    })
}

fn parse_function_body_bullet(function: &mut AilFunction, bullet: &str) {
    if let Some(text) = bullet.strip_prefix("if ") {
        let text = trim_sentence(text);
        if let Some((condition, return_value)) = text.split_once(", the function returns ") {
            function.branches.push(condition.trim().to_string());
            function.returns.push(trim_sentence(return_value));
        } else {
            function.branches.push(text);
        }
    } else if let Some(text) = bullet.strip_prefix("otherwise the function calls ") {
        let text = trim_sentence(text);
        function.calls.push(AilFunctionCall {
            target: function_call_target(&text),
            provenance: format!("function:{}.call:{text}", function.name),
            text,
        });
    } else if let Some(text) = bullet.strip_prefix("the function calls ") {
        let text = trim_sentence(text);
        function.calls.push(AilFunctionCall {
            target: function_call_target(&text),
            provenance: format!("function:{}.call:{text}", function.name),
            text,
        });
    } else if let Some(text) = bullet.strip_prefix("the function has ") {
        let text = trim_sentence(text);
        if function_termination_bound_text(&text) {
            function.termination_bounds.push(text);
        } else if function_termination_measure_text(&text) {
            function.termination_measures.push(text);
        }
    } else if let Some(text) = bullet.strip_prefix("the function returns ") {
        function.returns.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the function records a trace event named ") {
        function.traces.push(trim_sentence(text));
    }
}

fn function_termination_bound_text(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("recursion depth")
        || text.contains("stack bound")
        || text.contains("stack depth")
        || text.contains("termination bound")
}

fn function_termination_measure_text(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    (text.contains("termination measure") || text.contains("well-founded measure"))
        && text.contains("decreas")
}

fn parse_external_binding_bullet(
    document: &mut AilDocument,
    binding_name: &str,
    section: ExternalBindingSection,
    bullet: &str,
    line_number: usize,
) -> Result<(), String> {
    let binding = document
        .external_bindings
        .get_mut(binding_name)
        .ok_or_else(|| format!("line {line_number}: unknown external binding {binding_name}"))?;
    match section {
        ExternalBindingSection::Inputs => {
            let value = parse_external_binding_value(binding_name, "input", bullet, line_number)?;
            binding.inputs.insert(value.name.clone(), value);
        }
        ExternalBindingSection::Outputs => {
            let value = parse_external_binding_value(binding_name, "output", bullet, line_number)?;
            binding.outputs.insert(value.name.clone(), value);
        }
        ExternalBindingSection::StatusMaps => {
            let Some((code, target)) = bullet.split_once(" maps to ") else {
                return Err(format!(
                    "line {line_number}: expected '<code> maps to <target>'"
                ));
            };
            let code = code.trim().to_string();
            binding.status_maps.push(AilExternalStatusMap {
                provenance: format!("external_binding:{binding_name}.status:{code}"),
                code,
                target: trim_sentence(target),
            });
        }
        ExternalBindingSection::Capabilities => binding.capabilities.push(trim_sentence(bullet)),
        ExternalBindingSection::Traces => binding.traces.push(trim_sentence(bullet)),
    }
    Ok(())
}

fn parse_external_binding_value(
    binding_name: &str,
    role: &str,
    bullet: &str,
    line_number: usize,
) -> Result<AilExternalBindingValue, String> {
    let (name, type_and_ownership) = parse_typed_bullet(bullet, line_number)?;
    let (type_name, ownership) = split_external_type_and_ownership(&type_and_ownership);
    Ok(AilExternalBindingValue {
        name: name.clone(),
        type_name: normalize_type_name(&type_name),
        ownership,
        provenance: format!("external_binding:{binding_name}.{role}:{name}"),
    })
}

fn split_external_type_and_ownership(type_and_ownership: &str) -> (String, String) {
    let trimmed = type_and_ownership.trim();
    if let Some(end) = trimmed.rfind('>')
        && trimmed[..=end].contains('<')
    {
        let type_name = trimmed[..=end].trim().to_string();
        let ownership = trimmed[end + 1..].trim().to_string();
        return (type_name, ownership);
    }
    if let Some((type_name, ownership)) = trimmed.split_once(' ') {
        (type_name.trim().to_string(), ownership.trim().to_string())
    } else {
        (trimmed.to_string(), String::new())
    }
}

fn render_external_binding_value_type(value: &AilExternalBindingValue) -> String {
    if value.ownership.is_empty() {
        value.type_name.clone()
    } else {
        format!("{} {}", value.type_name, value.ownership)
    }
}

fn render_variant_spec(variant: &AilVariant) -> String {
    if variant.fields.is_empty() {
        return variant.label.clone();
    }
    let fields = variant
        .fields
        .values()
        .map(|field| format!("{}: {}", field.name, field.type_name))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({fields})", variant.label)
}

fn external_binding_value_attributes(value: &AilExternalBindingValue) -> BTreeMap<String, String> {
    if value.ownership.is_empty() {
        BTreeMap::new()
    } else {
        attr(&[("ownership", &value.ownership)])
    }
}

fn external_status_map_failure(target: &str) -> Option<&str> {
    target
        .strip_prefix("Failure.")
        .or_else(|| (target != "success").then_some(target))
}

fn function_call_target(text: &str) -> String {
    text.split_once(" with ")
        .map(|(target, _)| target.trim())
        .unwrap_or(text.trim())
        .to_string()
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
    } else if let Some(text) = bullet.strip_prefix("Requires ") {
        action
            .requirements
            .push(normalize_llm_requirement_shorthand(text));
    } else if let Some(text) = bullet.strip_prefix("the system reads ") {
        action.reads.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system changes ") {
        action.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("Changes ") {
        action.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system increments ") {
        action
            .writes
            .push(format!("increments {}", trim_sentence(text)));
    } else if let Some(text) = bullet.strip_prefix("the system decrements ") {
        action
            .writes
            .push(format!("decrements {}", trim_sentence(text)));
    } else if let Some(text) = bullet.strip_prefix("the system deletes ") {
        action
            .writes
            .push(format!("deletes {}", trim_sentence(text)));
    } else if let Some(text) = bullet.strip_prefix("the system removes ") {
        action
            .writes
            .push(format!("removes {}", trim_sentence(text)));
    } else if let Some(text) = bullet.strip_prefix("the system creates ") {
        action.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system calls ") {
        action.calls.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system repeats ") {
        if let Some(repeated_call) = parse_repeated_action_call(action_name, text) {
            action.repeated_calls.push(repeated_call);
        }
    } else if let Some(text) = bullet.strip_prefix("the system records a trace event named ") {
        action.traces.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("Records trace event named ") {
        action.traces.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system records ") {
        action.writes.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system guarantees ") {
        action.guarantees.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system claims scheduler behavior for ") {
        action
            .guarantees
            .push(format!("scheduler behavior for {}", trim_sentence(text)));
    } else if let Some(text) = bullet.strip_prefix("the system uses temporal policy ") {
        action
            .guarantees
            .push(format!("temporal policy {}", trim_sentence(text)));
    } else if let Some(text) = bullet.strip_prefix("the system uses retry policy ") {
        action
            .guarantees
            .push(format!("retry policy {}", trim_sentence(text)));
    } else if let Some(text) = bullet.strip_prefix("the system uses backoff policy ") {
        action
            .guarantees
            .push(format!("backoff policy {}", trim_sentence(text)));
    } else if let Some(text) = bullet.strip_prefix("Guarantees ") {
        action.guarantees.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("the system does not reveal ") {
        action.secret_protections.push(trim_sentence(text));
    } else if let Some(text) = bullet.strip_prefix("if ") {
        action.failures.push(trim_sentence(text));
    }
}

fn normalize_llm_requirement_shorthand(text: &str) -> String {
    let text = trim_sentence(text);
    if let Some(subject) = text.strip_suffix(" exists") {
        return format!("the {} to exist", subject.trim());
    }
    text
}

fn parse_repeated_action_call(action_name: &str, text: &str) -> Option<AilRepeatedActionCall> {
    let text = trim_sentence(text);
    let text = text.strip_suffix(" times").unwrap_or(&text);
    let (target, count) = text.rsplit_once(' ')?;
    let count = count.parse::<usize>().ok()?;
    (count > 0).then(|| AilRepeatedActionCall {
        target: target.trim().to_string(),
        count,
        provenance: format!("action:{action_name}.repeat:{}", target.trim()),
    })
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
    if let Some(inner) = type_name.strip_prefix("Pointer ") {
        return format!("Pointer<{}>", normalize_type_name(inner));
    }
    if let Some(inner) = type_name.strip_prefix("Nullable ") {
        return format!("Nullable<{}>", normalize_type_name(inner));
    }
    if let Some(inner) = type_name.strip_prefix("NonNull ") {
        return format!("NonNull<{}>", normalize_type_name(inner));
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
    for wrapper in ["Secret", "List", "Option", "Pointer", "Nullable", "NonNull"] {
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

fn resolve_action_call_target(
    graph: &Graph,
    document: &AilDocument,
    text: &str,
) -> Option<crate::core_model::Node> {
    let target_name = action_call_target_name(document, text)?;
    graph.find_node("Action", &target_name).cloned()
}

fn action_call_target_name(document: &AilDocument, text: &str) -> Option<String> {
    let trimmed = trim_sentence(text);
    if document.actions.contains_key(&trimmed) {
        return Some(trimmed);
    }
    let pascal = action_name_from_label(&trimmed);
    if document.actions.contains_key(&pascal) {
        return Some(pascal);
    }
    let compact = compact_requirement_match_text(&trimmed);
    document.actions.values().find_map(|action| {
        let compact_name = compact_requirement_match_text(&action.name);
        let compact_label = compact_requirement_match_text(&action.label);
        (compact == compact_name || compact == compact_label).then(|| action.name.clone())
    })
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
enum FunctionSection {
    Inputs,
    Outputs,
    Body,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExternalBindingSection {
    Inputs,
    Outputs,
    StatusMaps,
    Capabilities,
    Traces,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeSection {
    Variants,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteSection {
    Path,
    Reads,
    Permissions,
    Traces,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormSection {
    Action,
    Fields,
    Validations,
    FailureTraces,
    Confirmations,
    Accessibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DashboardSection {
    Reads,
    Permissions,
    Filters,
    Traces,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowSection {
    Steps,
    Blocks,
    Traces,
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
        || parse_function_header(line).is_some()
        || parse_function_section(line).is_some()
        || parse_function_body_header(line).is_some()
        || parse_type_header(line).is_some()
        || parse_route_header(line).is_some()
        || parse_route_section(line).is_some()
        || parse_form_header(line).is_some()
        || parse_form_section(line).is_some()
        || parse_dashboard_header(line).is_some()
        || parse_dashboard_section(line).is_some()
        || parse_workflow_header(line).is_some()
        || parse_workflow_section(line).is_some()
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
