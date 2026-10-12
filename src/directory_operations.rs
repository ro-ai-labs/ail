//! Caller-bound directory operations with checked relative paths.
use super::*;
use crate::expression::IntExpr;

const MAX_RESULT_BYTES: usize = 1_048_576;
const MAX_ALLOCATED_BYTES: usize = 67_108_864;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilDirectoryOperation {
    ListNames {
        resource: String,
        path: String,
        key: String,
    },
    ReadText {
        resource: String,
        path: String,
        key: String,
    },
    WriteText {
        resource: String,
        path: String,
        source: String,
    },
    Invalid(String),
}
#[derive(Default, Clone, Debug)]
pub struct AilDirectoryResources {
    pub reads: BTreeMap<String, PathBuf>,
    pub writes: BTreeMap<String, PathBuf>,
}
impl AilDirectoryOperation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::ListNames {
                resource,
                path,
                key,
            } => format!(
                "the system lists names in directory resource {resource} at {path} into {key}"
            ),
            Self::ReadText {
                resource,
                path,
                key,
            } => format!(
                "the system reads text from directory resource {resource} at {path} into {key}"
            ),
            Self::WriteText {
                resource,
                path,
                source,
            } => format!(
                "the system writes text {source} to directory resource {resource} at {path}"
            ),
            Self::Invalid(message) => format!("invalid directory operation: {message}"),
        }
    }
}
fn error(message: impl Display) -> String {
    format!("AIL-DIRECTORY-001: {message}")
}
fn field(key: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(key), Ok(IntExpr::Field(parsed)) if parsed == key) {
        Ok(())
    } else {
        Err(error("directory operand must be a field reference"))
    }
}
fn valid_path(path: &Path) -> bool {
    let bytes = path.as_os_str().as_encoded_bytes();
    path.is_absolute() && !bytes.is_empty() && bytes.len() <= 4095 && !bytes.contains(&0)
}
/// Parse an explicit resource=absolute-root directory grant without opening it.
pub fn parse_binding(binding: &str) -> Result<(String, PathBuf), String> {
    let (resource, path) = binding
        .split_once('=')
        .ok_or("AIL-RUNTIME-CAPABILITY: directory binding requires resource=absolute-root")?;
    if !file_operations::valid_resource(resource) || !valid_path(Path::new(path)) {
        return Err("AIL-RUNTIME-CAPABILITY: invalid directory resource binding".into());
    }
    Ok((resource.into(), path.into()))
}
pub(super) fn validate_resources(resources: &AilDirectoryResources) -> Result<(), String> {
    for (resource, path) in resources.reads.iter().chain(&resources.writes) {
        if !file_operations::valid_resource(resource) || !valid_path(path) {
            return Err("AIL-RUNTIME-CAPABILITY: invalid directory resource binding".into());
        }
    }
    Ok(())
}
fn boundary(text: &str, prefix: &str) -> bool {
    text.strip_prefix(prefix).is_some_and(|tail| {
        tail.chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_')
    })
}
pub(super) fn recognized_bullet(text: &str) -> bool {
    let text = trim_sentence(text);
    let Some(body) = text
        .strip_prefix("the system ")
        .or_else(|| text.strip_prefix("the function "))
    else {
        return false;
    };
    boundary(body, "lists names in directory")
        || boundary(body, "reads text from directory")
        || body
            .strip_prefix("writes text ")
            .and_then(|s| {
                s.strip_prefix("to ")
                    .or_else(|| s.split_once(" to ").map(|(_, tail)| tail))
            })
            .is_some_and(|tail| boundary(tail, "directory"))
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilDirectoryOperation, String>> {
    let text = trim_sentence(text);
    if !text.starts_with("the system ") || !recognized_bullet(&text) {
        return None;
    }
    Some((|| {
        fn split<'a>(s: &'a str, separator: &str) -> Result<(&'a str, &'a str), String> {
            s.split_once(separator)
                .ok_or_else(|| error("malformed directory operation"))
        }
        let op = if let Some(rest) =
            text.strip_prefix("the system lists names in directory resource ")
        {
            let (resource, rest) = split(rest, " at ")?;
            let (path, key) = split(rest, " into ")?;
            AilDirectoryOperation::ListNames {
                resource: resource.into(),
                path: path.into(),
                key: key.into(),
            }
        } else if let Some(rest) =
            text.strip_prefix("the system reads text from directory resource ")
        {
            let (resource, rest) = split(rest, " at ")?;
            let (path, key) = split(rest, " into ")?;
            AilDirectoryOperation::ReadText {
                resource: resource.into(),
                path: path.into(),
                key: key.into(),
            }
        } else if let Some(rest) = text.strip_prefix("the system writes text ") {
            let (source, rest) = split(rest, " to directory resource ")?;
            let (resource, path) = split(rest, " at ")?;
            AilDirectoryOperation::WriteText {
                resource: resource.into(),
                path: path.into(),
                source: source.into(),
            }
        } else {
            return Err(error("malformed directory operation"));
        };
        verify(&raw_instruction(&op)?)?;
        Ok(op)
    })())
}
// Kind, opcode, last operand name, edge role and field type.
fn schema(kind: &str) -> Result<(&'static str, &'static str, &'static str, &'static str), String> {
    match kind {
        "list_names" => Ok((
            "DIRECTORY_LIST_NAMES",
            "key",
            "directory_destination",
            "List<Text>",
        )),
        "read_text" => Ok((
            "DIRECTORY_READ_TEXT",
            "key",
            "directory_destination",
            "Text",
        )),
        "write_text" => Ok(("DIRECTORY_WRITE_TEXT", "source", "directory_source", "Text")),
        _ => Err(error("unknown directory operation kind")),
    }
}
fn components(op: &AilDirectoryOperation) -> Result<(&str, &str, &str, &str), String> {
    match op {
        AilDirectoryOperation::ListNames {
            resource,
            path,
            key,
        } => Ok(("list_names", resource, path, key)),
        AilDirectoryOperation::ReadText {
            resource,
            path,
            key,
        } => Ok(("read_text", resource, path, key)),
        AilDirectoryOperation::WriteText {
            resource,
            path,
            source,
        } => Ok(("write_text", resource, path, source)),
        AilDirectoryOperation::Invalid(message) => Err(error(message)),
    }
}
fn operation(
    kind: &str,
    resource: String,
    path: String,
    value: String,
) -> Result<AilDirectoryOperation, String> {
    match kind {
        "list_names" => Ok(AilDirectoryOperation::ListNames {
            resource,
            path,
            key: value,
        }),
        "read_text" => Ok(AilDirectoryOperation::ReadText {
            resource,
            path,
            key: value,
        }),
        "write_text" => Ok(AilDirectoryOperation::WriteText {
            resource,
            path,
            source: value,
        }),
        _ => Err(error("unknown directory operation kind")),
    }
}
fn raw_instruction(op: &AilDirectoryOperation) -> Result<AilBytecodeInstruction, String> {
    let (kind, resource, path, value) = components(op)?;
    let (opcode, name, _, _) = schema(kind)?;
    Ok(AilBytecodeInstruction::new(
        opcode,
        &[
            ("resource", resource.into()),
            ("path", path.into()),
            (name, value.into()),
        ],
    ))
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, op) in action.directory_operations.iter().enumerate() {
            let parts = components(op);
            let (kind, resource) = parts
                .as_ref()
                .map(|p| (p.0, p.1))
                .unwrap_or(("invalid", ""));
            let node = graph.add_node(
                "DirectoryOperation",
                format!("{}.directory{index}", owner.name),
                None,
                attr(&[("directory_kind", kind), ("resource", resource)]),
            );
            attach_provenance(
                graph,
                &node,
                format!("action:{}.directory:{index}", owner.name),
            );
            if let Ok((kind, _, path, value)) = parts {
                let (_, _, role, _) = schema(kind).expect("AST directory kind");
                for (edge_kind, key) in [("directory_path", path), (role, value)] {
                    let target = graph
                        .nodes
                        .iter()
                        .find(|n| {
                            n.kind == "Field"
                                && !n.attributes.contains_key("function_owner")
                                && !n.attributes.contains_key("local_name")
                                && expression_core::field_key(n) == key
                        })
                        .cloned()
                        .unwrap_or_else(|| {
                            let target = graph.add_node("Effect", key, None, BTreeMap::new());
                            attach_provenance(
                                graph,
                                &target,
                                format!("action:{}.directory:{index}.{edge_kind}", owner.name),
                            );
                            target
                        });
                    graph.add_edge(edge_kind, &node, &target, BTreeMap::new());
                }
            }
            action_order::append(
                graph,
                "directory_step",
                &owner,
                &node,
                BTreeMap::new(),
                index,
            );
        }
    }
}
fn executable_action(node: &Node) -> bool {
    node.kind == "Action"
        && node
            .attributes
            .get("kind")
            .is_none_or(|k| k != "CompilerPass")
}
fn valid_step(edge: &Edge) -> bool {
    edge.attributes
        .get("step_index")
        .is_some_and(|s| s.parse::<usize>().is_ok())
        && edge.attributes.iter().all(|(k, v)| {
            matches!(k.as_str(), "step_index" | "operation_index") && v.parse::<usize>().is_ok()
        })
}
fn unique_node<'a>(core: &'a AilCore, id: &str) -> Result<&'a Node, String> {
    let mut found = core.graph.nodes.iter().filter(|n| n.id == id);
    let node = found
        .next()
        .ok_or_else(|| error("missing directory graph node"))?;
    if found.next().is_some() {
        return Err(error("directory graph node identity must be unique"));
    }
    Ok(node)
}
fn validate_provenance(core: &AilCore, node: &Node) -> Result<(), String> {
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == "has_provenance")
    {
        if !edge.attributes.is_empty() || unique_node(core, &edge.target)?.kind != "Provenance" {
            return Err(error(
                "directory participants require attribute-free provenance edges to unique Provenance nodes",
            ));
        }
    }
    Ok(())
}
fn decode_node(core: &AilCore, node: &Node) -> Result<AilDirectoryOperation, String> {
    unique_node(core, &node.id)?;
    validate_provenance(core, node)?;
    if node.kind != "DirectoryOperation"
        || node.type_name.is_some()
        || node
            .attributes
            .keys()
            .map(String::as_str)
            .ne(["directory_kind", "resource"])
    {
        return Err(error(
            "directory node requires exactly directory_kind and resource, without a type annotation",
        ));
    }
    let kind = &node.attributes["directory_kind"];
    let (_, _, role, field_type) = schema(kind)?;
    if !file_operations::valid_resource(&node.attributes["resource"]) {
        return Err(error("invalid directory resource name"));
    }
    let incoming = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect::<Vec<_>>();
    if incoming.len() != 1
        || incoming[0].kind != "directory_step"
        || !valid_step(incoming[0])
        || !executable_action(unique_node(core, &incoming[0].source)?)
    {
        return Err(error(
            "directory operation requires exactly one ordered executable-action owner",
        ));
    }
    validate_provenance(core, unique_node(core, &incoming[0].source)?)?;
    let mut fields = BTreeMap::new();
    for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
        if !edge.attributes.is_empty() {
            return Err(error(
                "directory operand and provenance edges require no attributes",
            ));
        }
        let target = unique_node(core, &edge.target)?;
        if edge.kind == "has_provenance" && target.kind == "Provenance" {
            continue;
        }
        let expected = if edge.kind == "directory_path" {
            "Text"
        } else if edge.kind == role {
            field_type
        } else {
            return Err(error("unexpected directory operand edge"));
        };
        let key = float_operations::scoped_field(core, target, None, expected).map_err(error)?;
        validate_provenance(core, target)?;
        for declaration in core
            .graph
            .edges
            .iter()
            .filter(|e| e.kind == "has_field" && e.target == target.id)
        {
            validate_provenance(core, unique_node(core, &declaration.source)?)?;
        }
        if fields.insert(edge.kind.as_str(), key).is_some() {
            return Err(error("duplicate directory operand edge"));
        }
    }
    if fields.len() != 2 {
        return Err(error(
            "directory operation requires exactly two field operands",
        ));
    }
    let op = operation(
        kind,
        node.attributes["resource"].clone(),
        fields["directory_path"].clone(),
        fields[role].clone(),
    )?;
    verify(&raw_instruction(&op)?)?;
    Ok(op)
}
pub(super) fn from_core(core: &AilCore, owner: &Node) -> Vec<AilDirectoryOperation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == owner.id && e.kind == "directory_step")
        .map(|e| {
            unique_node(core, &e.target)
                .and_then(|n| decode_node(core, n))
                .unwrap_or_else(AilDirectoryOperation::Invalid)
        })
        .collect()
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for node in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "DirectoryOperation")
    {
        let result = if matches!(core.package.profile.as_str(), "Application" | "C interop") {
            decode_node(core, node).map(|_| ())
        } else {
            Err(error(
                "directory operations require Application or C interop profile",
            ))
        };
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-DIRECTORY-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = unique_node(core, &edge.source).ok();
        let target = unique_node(core, &edge.target).ok();
        let valid = match edge.kind.as_str() {
            "directory_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "DirectoryOperation")
                    && valid_step(edge)
            }
            "directory_path" | "directory_source" | "directory_destination" => {
                source.is_some_and(|n| n.kind == "DirectoryOperation")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-DIRECTORY-001",
                    format!("invalid {} directory edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace(op: &AilDirectoryOperation, alias: &str) -> AilDirectoryOperation {
    let qualify = |value: &str| format!("{}.{value}", runtime_subject_key(alias));
    match components(op) {
        Ok((kind, resource, path, value)) => {
            operation(kind, qualify(resource), qualify(path), qualify(value))
                .expect("AST directory kind")
        }
        Err(_) => op.clone(),
    }
}
pub(super) fn instruction(
    op: &AilDirectoryOperation,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let instruction = raw_instruction(op)?;
    verify(&instruction)?;
    let (kind, _, _, _) = components(op)?;
    let (_, name, _, field_type) = schema(kind)?;
    for (name, expected) in [("path", "Text"), (name, field_type)] {
        if runtime_field_type_name(document, &instruction.operands[name]) != Some(expected) {
            return Err(error(format!(
                "directory {name} must be a declared global {expected} field"
            )));
        }
    }
    Ok(instruction)
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let names = match instruction.opcode.as_str() {
        "DIRECTORY_LIST_NAMES" | "DIRECTORY_READ_TEXT" => ["key", "path", "resource"],
        "DIRECTORY_WRITE_TEXT" => ["path", "resource", "source"],
        _ => return Err(error("unknown directory instruction")),
    };
    if instruction.operands.keys().map(String::as_str).ne(names) {
        return Err(error(
            "directory instruction requires exactly resource, path and its result or source field",
        ));
    }
    if !file_operations::valid_resource(&instruction.operands["resource"]) {
        return Err(error("invalid directory resource name"));
    }
    for (name, value) in &instruction.operands {
        if name != "resource" {
            field(value)?;
        }
    }
    Ok(())
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let roles: &[&str] = match kind {
        "reads" => &["directory_path", "directory_source"],
        "writes" => &["directory_destination"],
        _ => return Vec::new(),
    };
    let mut result = Vec::new();
    for step in core
        .graph
        .edges
        .iter()
        .filter(|e| e.kind == "directory_step")
    {
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|e| e.source == step.target && roles.contains(&e.kind.as_str()))
        {
            result.push((edge, step.source.as_str()));
        }
    }
    result
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn host_status(status: u64) -> Result<(), String> {
    match status {
        0 => Ok(()),
        1 => Err("AIL-RUNTIME-PATH: directory path is unavailable or invalid".into()),
        2 => Err("AIL-RUNTIME-TEXT: directory operation requires valid UTF-8 text".into()),
        3 => Err("AIL-RUNTIME-TEXT-LIMIT: directory text exceeds limits".into()),
        4 => Err("AIL-RUNTIME-LIST-LIMIT: directory listing exceeds limits".into()),
        _ => Err("AIL-RUNTIME-IO: directory host operation failed".into()),
    }
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn invoke(
    kind: u64,
    root: &Path,
    path: Option<&str>,
    source: Option<&str>,
) -> Result<String, String> {
    use crate::directory_host::{
        DirectoryRequest, DirectoryResult, ail_directory_run, ail_directory_validate,
    };
    const CAPACITY: usize = MAX_RESULT_BYTES + 1;
    const SCRATCH: usize = 0x311000;
    let input =
        |value: Option<&str>| value.map_or((std::ptr::null(), 0), |s| (s.as_ptr(), s.len() as u64));
    let (path, path_len) = input(path);
    let (source, source_len) = input(source);
    let mut request = DirectoryRequest {
        kind,
        root: std::ptr::null(),
        path,
        path_len,
        source,
        source_len,
        output: std::ptr::null_mut(),
        output_capacity: 0,
        scratch: std::ptr::null_mut(),
        scratch_capacity: 0,
    };
    // The validator reads only immutable field slices, including null for missing fields.
    host_status(unsafe { ail_directory_validate(&request) })?;
    let io_error = || "AIL-RUNTIME-IO: directory host allocation failed".to_string();
    let mut root_bytes = Vec::new();
    root_bytes
        .try_reserve_exact(root.as_os_str().as_encoded_bytes().len() + 1)
        .map_err(|_| io_error())?;
    root_bytes.extend_from_slice(root.as_os_str().as_encoded_bytes());
    root_bytes.push(0);
    let mut output = Vec::new();
    output.try_reserve_exact(CAPACITY).map_err(|_| io_error())?;
    output.resize(CAPACITY, 0);
    // u64 storage guarantees the descriptor regions' required eight-byte alignment.
    let mut scratch = Vec::<u64>::new();
    scratch
        .try_reserve_exact(SCRATCH / 8)
        .map_err(|_| io_error())?;
    scratch.resize(SCRATCH / 8, 0);
    request.root = root_bytes.as_ptr();
    request.output = output.as_mut_ptr();
    request.output_capacity = CAPACITY as u64;
    request.scratch = scratch.as_mut_ptr().cast();
    request.scratch_capacity = SCRATCH as u64;
    let mut result = DirectoryResult {
        output_len: 0,
        entry_count: 0,
    };
    // Owned buffers are disjoint; all input references and the terminated root live through the call.
    host_status(unsafe { ail_directory_run(&request, &mut result) })?;
    if result.output_len > MAX_RESULT_BYTES as u64
        || result.entry_count > 65_536
        || (kind != 0 && result.entry_count != 0)
        || (kind == 2 && result.output_len != 0)
    {
        return Err("AIL-RUNTIME-IO: invalid directory host result".into());
    }
    output.truncate(result.output_len as usize);
    String::from_utf8(output).map_err(|_| "AIL-RUNTIME-TEXT: directory result is not UTF-8".into())
}
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn invoke(_: u64, _: &Path, _: Option<&str>, _: Option<&str>) -> Result<String, String> {
    Err("AIL-RUNTIME-IO: directory operations require Linux x86-64 with openat2".into())
}
pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
    text_allocated: &mut usize,
    list_allocated: &mut usize,
    resources: &AilDirectoryResources,
) -> Result<(), String> {
    verify(instruction)?;
    let kind = match instruction.opcode.as_str() {
        "DIRECTORY_LIST_NAMES" => 0,
        "DIRECTORY_READ_TEXT" => 1,
        _ => 2,
    };
    let grants = if kind == 2 {
        &resources.writes
    } else {
        &resources.reads
    };
    let root = grants
        .get(&instruction.operands["resource"])
        .ok_or("AIL-RUNTIME-CAPABILITY: missing directory resource grant")?;
    let path = state.get(&instruction.operands["path"]).map(String::as_str);
    let source = instruction
        .operands
        .get("source")
        .and_then(|key| state.get(key))
        .map(String::as_str);
    let value = invoke(kind, root, path, source)?;
    if kind == 2 {
        return Ok(());
    }
    if kind == 0 {
        if value.len() > MAX_RESULT_BYTES {
            return Err("AIL-RUNTIME-LIST-LIMIT: directory listing exceeds limits".into());
        }
        let allocated = list_allocated
            .checked_add(value.len())
            .filter(|n| *n <= MAX_ALLOCATED_BYTES)
            .ok_or("AIL-RUNTIME-LIST-LIMIT: directory listing allocation budget exceeded")?;
        state.insert(instruction.operands["key"].clone(), value);
        *list_allocated = allocated;
    } else {
        let mut allocated = *text_allocated;
        text_operations::reserve_result(value.len(), &mut allocated)?;
        state.insert(instruction.operands["key"].clone(), value);
        *text_allocated = allocated;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const FORMS: [&str; 3] = [
        "the system lists names in directory resource tree at data.path into data.names",
        "the system reads text from directory resource tree at data.path into data.body",
        "the system writes text data.body to directory resource tree at data.path",
    ];
    fn operations() -> Vec<AilDirectoryOperation> {
        vec![
            AilDirectoryOperation::ListNames {
                resource: "tree".into(),
                path: "data.path".into(),
                key: "data.names".into(),
            },
            AilDirectoryOperation::ReadText {
                resource: "tree".into(),
                path: "data.path".into(),
                key: "data.body".into(),
            },
            AilDirectoryOperation::WriteText {
                resource: "tree".into(),
                path: "data.path".into(),
                source: "data.body".into(),
            },
        ]
    }
    fn command(index: usize) -> AilBytecodeInstruction {
        let (opcode, key, value) = match index {
            0 => ("DIRECTORY_LIST_NAMES", "key", "data.names"),
            1 => ("DIRECTORY_READ_TEXT", "key", "data.body"),
            _ => ("DIRECTORY_WRITE_TEXT", "source", "data.body"),
        };
        AilBytecodeInstruction::new(
            opcode,
            &[
                ("resource", "tree".into()),
                ("path", "data.path".into()),
                (key, value.into()),
            ],
        )
    }
    fn core() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Directory manages files.\nA Data has:\n- path: Text\n- body: Text\n- names: List<Text>\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Done\n";
        let mut document = parse_ail_package_spec_text(&package, source).unwrap();
        let action = document.actions.get_mut("Run").unwrap();
        action.directory_operations = operations();
        for index in 0..3 {
            action.steps.push(AilActionStep::Directory(index));
        }
        elaborate_ail_core(&package, &document)
    }
    fn rejected(core: &AilCore) {
        assert!(!check(core).is_empty());
        assert!(compile_ail_core_bytecode(core).is_err());
        if let Ok(saved) = parse_ail_core_text(&render_ail_core(core)) {
            assert!(!check(&saved).is_empty());
            assert!(compile_ail_core_bytecode(&saved).is_err());
        }
    }
    #[test]
    fn exact_forms_roundtrip_and_namespace_only_references() {
        for (op, form) in operations().iter().zip(FORMS) {
            assert_eq!(parse_bullet(form).unwrap().unwrap(), *op);
            assert_eq!(op.source(), form);
            assert_eq!(
                namespace(op, "Child").source(),
                form.replace("resource tree", "resource child.tree")
                    .replace("data.", "child.data.")
            );
        }
    }
    #[test]
    fn reserved_forms_are_recognized_without_claiming_other_effects() {
        for prefix in ["the system", "the function"] {
            for suffix in [
                "lists names in directory",
                "lists names in directory:",
                "reads text from directory resource",
                "writes text to directory",
                "writes text data.body to directory",
                "writes text data.body to directory resource tree",
            ] {
                assert!(recognized_bullet(&format!("{prefix} {suffix}")), "{suffix}");
            }
        }
        for text in [
            "the system writes text data.body to file resource tree",
            "the system writes text data.body to standard output",
            "the system lists names in directoryish work",
        ] {
            assert!(!recognized_bullet(text));
            assert!(parse_bullet(text).is_none());
        }
    }
    #[test]
    fn protocol_requires_exact_bare_fields_and_static_resource() {
        for n in 0..3 {
            let good = command(n);
            verify(&good).unwrap();
            for (key, value) in [
                ("hidden", "ignored"),
                ("path", "data.path + 1"),
                ("resource", "bad..name"),
            ] {
                let mut bad = good.clone();
                bad.operands.insert(key.into(), value.into());
                assert!(verify(&bad).is_err());
            }
            let mut bad = good;
            bad.operands.remove("path");
            assert!(verify(&bad).is_err());
        }
    }
    #[test]
    fn grants_are_absolute_bounded_and_all_entries_validate() {
        assert_eq!(
            parse_binding("Child.tree=/tmp/a=b").unwrap(),
            ("Child.tree".into(), PathBuf::from("/tmp/a=b"))
        );
        let root = format!("/{}", "x".repeat(4094));
        assert!(parse_binding(&format!("tree={root}")).is_ok());
        for binding in [
            "tree=relative".into(),
            "tree=".into(),
            "bad-name=/tmp".into(),
            "tree=/tmp/\0x".into(),
            format!("tree={root}x"),
        ] {
            assert!(
                parse_binding(&binding)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-CAPABILITY:")
            );
        }
        let resources = AilDirectoryResources {
            reads: BTreeMap::new(),
            writes: BTreeMap::from([("unused".into(), "relative".into())]),
        };
        assert!(validate_resources(&resources).is_err());
    }
    #[test]
    fn core_roundtrips_all_three_operations_and_semantic_accesses() {
        let original = core();
        assert_eq!(
            original
                .graph
                .nodes
                .iter()
                .filter(|n| n.kind == "DirectoryOperation")
                .count(),
            3
        );
        for candidate in [
            original.clone(),
            parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        ] {
            assert!(
                check_ail_core(&candidate).is_empty(),
                "{:?}",
                check_ail_core(&candidate)
            );
            let action = candidate
                .graph
                .nodes
                .iter()
                .find(|n| n.kind == "Action" && n.name == "Run")
                .unwrap();
            assert_eq!(from_core(&candidate, action), operations());
            assert_eq!(accesses(&candidate, "reads").len(), 4);
            assert_eq!(accesses(&candidate, "writes").len(), 2);
            let program = compile_ail_core_bytecode(&candidate).unwrap();
            assert_eq!(
                program.actions["Run"]
                    .instructions
                    .iter()
                    .filter(|i| i.opcode.starts_with("DIRECTORY_"))
                    .count(),
                3
            );
        }
    }
    #[test]
    fn core_rejects_duplicate_roles_metadata_and_ambiguous_identities() {
        let original = core();
        for role in [
            "directory_step",
            "directory_path",
            "directory_destination",
            "directory_source",
        ] {
            let mut bad = original.clone();
            let edge = bad
                .graph
                .edges
                .iter()
                .find(|e| e.kind == role)
                .unwrap()
                .clone();
            bad.graph.edges.push(edge);
            rejected(&bad);
            let mut bad = original.clone();
            bad.graph
                .edges
                .iter_mut()
                .find(|e| e.kind == role)
                .unwrap()
                .attributes
                .insert("hidden".into(), "ignored".into());
            rejected(&bad);
        }
        for kind in [
            "DirectoryOperation",
            "Action",
            "Field",
            "Thing",
            "Provenance",
        ] {
            let mut bad = original.clone();
            let node = if kind == "Provenance" {
                let operation = bad
                    .graph
                    .nodes
                    .iter()
                    .find(|n| n.kind == "DirectoryOperation")
                    .unwrap();
                let target = &bad
                    .graph
                    .edges
                    .iter()
                    .find(|e| e.source == operation.id && e.kind == "has_provenance")
                    .unwrap()
                    .target;
                bad.graph.nodes.iter().find(|n| n.id == *target).unwrap()
            } else {
                bad.graph.nodes.iter().find(|n| n.kind == kind).unwrap()
            }
            .clone();
            bad.graph.nodes.push(node);
            rejected(&bad);
        }
    }
    #[test]
    fn every_operand_requires_its_declared_type_and_unique_global_storage() {
        let original = core();
        assert_eq!(
            original
                .graph
                .edges
                .iter()
                .filter(|e| e.kind == "directory_path")
                .count(),
            3
        );
        for edge in original.graph.edges.iter().filter(|e| {
            matches!(
                e.kind.as_str(),
                "directory_path" | "directory_source" | "directory_destination"
            )
        }) {
            for change in ["type", "orphan", "local", "shadow", "declaration"] {
                let mut bad = original.clone();
                let field = bad
                    .graph
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == edge.target)
                    .unwrap();
                match change {
                    "type" => field.type_name = Some("Bool".into()),
                    "local" => {
                        field
                            .attributes
                            .insert("function_owner".into(), "foreign".into());
                    }
                    "orphan" => bad
                        .graph
                        .edges
                        .retain(|e| e.kind != "has_field" || e.target != edge.target),
                    "shadow" => {
                        let mut shadow = field.clone();
                        shadow.id = "shadow-field".into();
                        bad.graph.nodes.push(shadow);
                    }
                    _ => {
                        let declaration = bad
                            .graph
                            .edges
                            .iter()
                            .find(|e| e.kind == "has_field" && e.target == edge.target)
                            .unwrap()
                            .clone();
                        bad.graph.edges.push(declaration);
                    }
                }
                rejected(&bad);
            }
        }
    }
    #[test]
    fn every_participant_provenance_requires_unique_typed_target_and_empty_metadata() {
        let original = core();
        for kind in ["DirectoryOperation", "Action", "Field", "Thing"] {
            let owner = original
                .graph
                .nodes
                .iter()
                .find(|n| n.kind == kind)
                .unwrap();
            let edge = original
                .graph
                .edges
                .iter()
                .find(|e| e.source == owner.id && e.kind == "has_provenance")
                .unwrap();
            for mutation in ["identity", "kind", "metadata"] {
                let mut changed = original.clone();
                let index = changed
                    .graph
                    .nodes
                    .iter()
                    .position(|n| n.id == edge.target)
                    .unwrap();
                match mutation {
                    "identity" => changed.graph.nodes.push(changed.graph.nodes[index].clone()),
                    "kind" => changed.graph.nodes[index].kind = "Effect".into(),
                    _ => {
                        changed
                            .graph
                            .edges
                            .iter_mut()
                            .find(|e| e.id == edge.id)
                            .unwrap()
                            .attributes
                            .insert("hidden".into(), "ignored".into());
                    }
                }
                rejected(&changed);
            }
        }
    }
    #[test]
    fn strict_node_metadata_profiles_owners_and_semantic_secret_checks() {
        let original = core();
        for (attribute, value) in [
            ("directory_kind", "remove"),
            ("resource", "bad..name"),
            ("hidden", "data.path"),
        ] {
            let mut bad = original.clone();
            bad.graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == "DirectoryOperation")
                .unwrap()
                .attributes
                .insert(attribute.into(), value.into());
            rejected(&bad);
        }
        let mut bad = original.clone();
        bad.graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "DirectoryOperation")
            .unwrap()
            .type_name = Some("Text".into());
        rejected(&bad);
        for profile in ["System", "Compiler", "UI", "AgentTool"] {
            let mut bad = original.clone();
            bad.package.profile = profile.into();
            rejected(&bad);
        }
        let mut bad = original.clone();
        bad.graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Action" && n.name == "Run")
            .unwrap()
            .attributes
            .insert("kind".into(), "CompilerPass".into());
        rejected(&bad);
        let mut secret = original;
        secret
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        for node in secret.graph.nodes.iter_mut().filter(|n| n.kind == "Field") {
            node.attributes.insert("secret".into(), "true".into());
        }
        let errors = check_ail_core(&secret);
        assert_eq!(errors.iter().filter(|e| e.contains("AIL005")).count(), 4);
        assert_eq!(errors.iter().filter(|e| e.contains("AIL002")).count(), 2);
    }
    #[test]
    fn capability_precedes_missing_path_and_permissions_are_separate() {
        let resources = AilDirectoryResources {
            reads: BTreeMap::from([("tree".into(), "/missing/directory".into())]),
            writes: BTreeMap::new(),
        };
        let mut state = BTreeMap::new();
        let (mut text, mut list) = (7, 9);
        assert!(
            execute(&command(2), &mut state, &mut text, &mut list, &resources)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-CAPABILITY:")
        );
        assert!(
            execute(&command(1), &mut state, &mut text, &mut list, &resources)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT:")
        );
        assert_eq!((text, list), (7, 9));
        assert!(state.is_empty());
    }
    #[test]
    fn path_validation_precedes_write_text_and_host_lookup() {
        let resources = AilDirectoryResources {
            reads: BTreeMap::new(),
            writes: BTreeMap::from([("tree".into(), "/missing/directory".into())]),
        };
        for path in [
            "",
            "../escape",
            "/absolute",
            "a//b",
            "a/./b",
            "a/../b",
            "a/",
            "x\0y",
        ] {
            let mut state = BTreeMap::from([("data.path".into(), path.into())]);
            assert!(
                execute(&command(2), &mut state, &mut 0, &mut 0, &resources)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-PATH:"),
                "{path:?}"
            );
        }
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn path_and_source_bounds_preserve_validation_priority() {
        let resources = AilDirectoryResources {
            reads: BTreeMap::new(),
            writes: BTreeMap::from([("tree".into(), "/missing/directory".into())]),
        };
        for (length, category) in [
            (4095, "TEXT"),
            (4096, "PATH"),
            (1_048_576, "PATH"),
            (1_048_577, "TEXT-LIMIT"),
        ] {
            let mut state = BTreeMap::from([("data.path".into(), "x".repeat(length))]);
            let before = state.clone();
            let (mut text, mut list) = (19, 23);
            let error =
                execute(&command(2), &mut state, &mut text, &mut list, &resources).unwrap_err();
            assert!(
                error.starts_with(&format!("AIL-RUNTIME-{category}:")),
                "length {length}: {error}"
            );
            assert_eq!(state, before);
            assert_eq!((text, list), (19, 23));
        }
        for (length, category) in [(1_048_576, "PATH"), (1_048_577, "TEXT-LIMIT")] {
            let mut state = BTreeMap::from([
                ("data.path".into(), "file".into()),
                ("data.body".into(), "x".repeat(length)),
            ]);
            let error = execute(&command(2), &mut state, &mut 0, &mut 0, &resources).unwrap_err();
            assert!(
                error.starts_with(&format!("AIL-RUNTIME-{category}:")),
                "length {length}: {error}"
            );
        }
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    struct Files(PathBuf);
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    impl Files {
        fn new() -> Self {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "ail-directory-module-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn grants(&self) -> AilDirectoryResources {
            AilDirectoryResources {
                reads: BTreeMap::from([("tree".into(), self.0.clone())]),
                writes: BTreeMap::from([("tree".into(), self.0.clone())]),
            }
        }
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    impl Drop for Files {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn list_names_are_sorted_exact_and_charge_only_list_arena() {
        use std::os::unix::fs::symlink;
        let files = Files::new();
        let names = [
            "λ", "a\nline", ".hidden", "z=\\name", "\u{85}", "nested", "link",
        ];
        for name in &names[..5] {
            fs::write(files.0.join(name), b"").unwrap();
        }
        fs::create_dir(files.0.join("nested")).unwrap();
        symlink("missing", files.0.join("link")).unwrap();
        let mut state = BTreeMap::from([
            ("data.path".into(), "".into()),
            ("data.names".into(), "old".into()),
        ]);
        let (mut text, mut list) = (13, 17);
        execute(
            &command(0),
            &mut state,
            &mut text,
            &mut list,
            &files.grants(),
        )
        .unwrap();
        let mut expected = names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        expected.sort();
        assert_eq!(
            list_operations::parse_text_items(&state["data.names"]).unwrap(),
            expected
        );
        assert_eq!(
            state["data.names"],
            format!(
                "[{}]",
                expected
                    .iter()
                    .map(|s| crate::core_model::json_string(s))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        );
        assert_eq!(text, 13);
        assert_eq!(list, 17 + state["data.names"].len());
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn relative_read_write_preserve_exact_bytes_and_path_alias() {
        let files = Files::new();
        fs::create_dir(files.0.join("nested")).unwrap();
        let mut state = BTreeMap::from([
            ("data.path".into(), "nested/a=b".into()),
            ("data.body".into(), "λ\0\r\n".into()),
            ("data.alias".into(), "unchanged".into()),
        ]);
        let (mut text, mut list) = (5, 7);
        execute(
            &command(2),
            &mut state,
            &mut text,
            &mut list,
            &files.grants(),
        )
        .unwrap();
        assert_eq!(
            fs::read(files.0.join("nested/a=b")).unwrap(),
            "λ\0\r\n".as_bytes()
        );
        assert_eq!((text, list), (5, 7));
        let mut read = command(1);
        read.operands.insert("key".into(), "data.path".into());
        execute(&read, &mut state, &mut text, &mut list, &files.grants()).unwrap();
        assert_eq!(state["data.path"], "λ\0\r\n");
        assert_eq!(state["data.alias"], "unchanged");
        assert_eq!((text, list), (10, 7));
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn failed_read_or_listing_preserves_destination_and_both_counters() {
        use std::os::unix::ffi::OsStringExt;
        let files = Files::new();
        fs::write(files.0.join("bad"), b"\xff").unwrap();
        let initial = BTreeMap::from([
            ("data.path".into(), "bad".into()),
            ("data.body".into(), "old body".into()),
            ("data.names".into(), "old names".into()),
        ]);
        let mut state = initial.clone();
        let (mut text, mut list) = (31, 37);
        assert!(
            execute(
                &command(1),
                &mut state,
                &mut text,
                &mut list,
                &files.grants()
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-TEXT:")
        );
        assert_eq!(state, initial);
        assert_eq!((text, list), (31, 37));
        fs::write(files.0.join(std::ffi::OsString::from_vec(vec![255])), b"").unwrap();
        state.insert("data.path".into(), "".into());
        let before = state.clone();
        assert!(
            execute(
                &command(0),
                &mut state,
                &mut text,
                &mut list,
                &files.grants()
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-TEXT:")
        );
        assert_eq!(state, before);
        assert_eq!((text, list), (31, 37));
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn read_sentinel_precedes_utf8_and_invalid_write_does_not_truncate() {
        let files = Files::new();
        fs::write(files.0.join("body"), b"previous bytes").unwrap();
        let mut state = BTreeMap::from([
            ("data.path".into(), "body".into()),
            ("data.body".into(), "x".repeat(1_048_577)),
        ]);
        assert!(
            execute(&command(2), &mut state, &mut 0, &mut 0, &files.grants())
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT-LIMIT:")
        );
        assert_eq!(fs::read(files.0.join("body")).unwrap(), b"previous bytes");
        state.remove("data.body");
        assert!(
            execute(&command(2), &mut state, &mut 0, &mut 0, &files.grants())
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT:")
        );
        assert_eq!(fs::read(files.0.join("body")).unwrap(), b"previous bytes");

        let mut bytes = vec![b'x'; 1_048_577];
        bytes[0] = 255;
        fs::write(files.0.join("body"), &bytes).unwrap();
        state.insert("data.body".into(), "old".into());
        let before = state.clone();
        let (mut text, mut list) = (29, 31);
        assert!(
            execute(
                &command(1),
                &mut state,
                &mut text,
                &mut list,
                &files.grants()
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-TEXT-LIMIT:")
        );
        assert_eq!(state, before);
        assert_eq!((text, list), (29, 31));
        bytes.truncate(1_048_576);
        bytes[0] = b'x';
        fs::write(files.0.join("body"), &bytes).unwrap();
        execute(
            &command(1),
            &mut state,
            &mut text,
            &mut list,
            &files.grants(),
        )
        .unwrap();
        assert_eq!(state["data.body"].as_bytes(), bytes);
        assert_eq!((text, list), (29 + 1_048_576, 31));
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn result_arenas_are_atomic_and_empty_values_have_distinct_costs() {
        let files = Files::new();
        let mut state = BTreeMap::from([
            ("data.path".into(), "".into()),
            ("data.names".into(), "old".into()),
        ]);
        let before = state.clone();
        let (mut text, mut list) = (0, 67_108_863);
        assert!(
            execute(
                &command(0),
                &mut state,
                &mut text,
                &mut list,
                &files.grants()
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
        assert_eq!(state, before);
        assert_eq!((text, list), (0, 67_108_863));
        list -= 1;
        execute(
            &command(0),
            &mut state,
            &mut text,
            &mut list,
            &files.grants(),
        )
        .unwrap();
        assert_eq!(state["data.names"], "[]");
        assert_eq!(list, 67_108_864);
        fs::write(files.0.join("empty"), b"").unwrap();
        state.insert("data.path".into(), "empty".into());
        state.insert("data.body".into(), "old".into());
        text = 67_108_864;
        let before = state.clone();
        assert!(
            execute(
                &command(1),
                &mut state,
                &mut text,
                &mut list,
                &files.grants()
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-TEXT-LIMIT:")
        );
        assert_eq!(state, before);
        assert_eq!((text, list), (67_108_864, 67_108_864));
        text -= 1;
        execute(
            &command(1),
            &mut state,
            &mut text,
            &mut list,
            &files.grants(),
        )
        .unwrap();
        assert_eq!(state["data.body"], "");
        assert_eq!(text, 67_108_864);
    }
}
