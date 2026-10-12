//! Explicit, caller-bound text and binary file effects.
use super::*;
use crate::expression::IntExpr;
use std::io::{Read, Write};

const MAX_TEXT_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilFileOperation {
    Read { resource: String, key: String },
    Write { resource: String, source: String },
    ReadBytes { resource: String, key: String },
    WriteBytes { resource: String, source: String },
    Invalid(String),
}

#[derive(Default, Clone, Debug)]
pub struct AilFileResources {
    pub reads: BTreeMap<String, PathBuf>,
    pub writes: BTreeMap<String, PathBuf>,
}

impl AilFileOperation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Read { resource, key } => {
                format!("the system reads text from file resource {resource} into {key}")
            }
            Self::Write { resource, source } => {
                format!("the system writes text {source} to file resource {resource}")
            }
            Self::ReadBytes { resource, key } => {
                format!("the system reads bytes from file resource {resource} into {key}")
            }
            Self::WriteBytes { resource, source } => {
                format!("the system writes bytes {source} to file resource {resource}")
            }
            Self::Invalid(message) => format!("invalid file operation: {message}"),
        }
    }
}

fn error(message: impl Display) -> String {
    format!("AIL-FILE-001: {message}")
}
fn field(key: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(key),Ok(IntExpr::Field(parsed)) if parsed==key) {
        Ok(())
    } else {
        Err(error("file operand must be a field reference"))
    }
}

/// Whether a static resource name is a bounded, optionally qualified ASCII identifier.
pub fn valid_resource(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.split('.').all(|part| {
            let mut bytes = part.bytes();
            bytes
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
                && bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_')
        })
}

/// Parse the `resource=path` portion of an explicit caller grant.
pub fn parse_binding(binding: &str) -> Result<(String, PathBuf), String> {
    let (resource, path) = binding
        .split_once('=')
        .ok_or("AIL-RUNTIME-CAPABILITY: file binding requires resource=path")?;
    if !valid_resource(resource) || path.is_empty() || path.contains('\0') {
        return Err("AIL-RUNTIME-CAPABILITY: invalid file resource binding".into());
    }
    Ok((resource.into(), PathBuf::from(path)))
}

/// Check all host bindings before execution, including unused ones.
pub(super) fn validate_resources(resources: &AilFileResources) -> Result<(), String> {
    for (name, path) in resources.reads.iter().chain(&resources.writes) {
        if !valid_resource(name)
            || path.as_os_str().is_empty()
            || path.as_os_str().as_encoded_bytes().contains(&0)
        {
            return Err("AIL-RUNTIME-CAPABILITY: invalid file resource binding".into());
        }
    }
    Ok(())
}

pub(super) fn recognized_binary_bullet(text: &str) -> bool {
    let text = trim_sentence(text);
    let Some(body) = text
        .strip_prefix("the system ")
        .or_else(|| text.strip_prefix("the function "))
    else {
        return false;
    };
    ["reads bytes", "writes bytes"].iter().any(|prefix| {
        body.strip_prefix(prefix).is_some_and(|tail| {
            tail.chars()
                .next()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_')
        })
    })
}

pub(super) fn parse_bullet(text: &str) -> Option<Result<AilFileOperation, String>> {
    let text = trim_sentence(text);
    let parsed = if let Some(rest) = text.strip_prefix("the system reads bytes from file resource ")
    {
        rest.split_once(" into ")
            .map(|(resource, key)| AilFileOperation::ReadBytes {
                resource: resource.into(),
                key: key.into(),
            })
    } else if let Some(rest) = text.strip_prefix("the system writes bytes ") {
        rest.split_once(" to file resource ")
            .map(|(source, resource)| AilFileOperation::WriteBytes {
                resource: resource.into(),
                source: source.into(),
            })
    } else if let Some(rest) = text.strip_prefix("the system reads text from file resource ") {
        rest.split_once(" into ")
            .map(|(resource, key)| AilFileOperation::Read {
                resource: resource.into(),
                key: key.into(),
            })
    } else if let Some(rest) = text.strip_prefix("the system writes text ") {
        rest.split_once(" to file resource ")
            .map(|(source, resource)| AilFileOperation::Write {
                resource: resource.into(),
                source: source.into(),
            })
    } else {
        return (text.starts_with("the system reads text from file resource")
            || text == "the system writes text"
            || recognized_binary_bullet(&text))
        .then(|| Err(error("malformed file operation")));
    };
    Some(
        parsed
            .ok_or_else(|| error("malformed file operation"))
            .and_then(|op| {
                let (resource, key) = match &op {
                    AilFileOperation::Read { resource, key }
                    | AilFileOperation::ReadBytes { resource, key } => (resource, key),
                    AilFileOperation::Write { resource, source }
                    | AilFileOperation::WriteBytes { resource, source } => (resource, source),
                    AilFileOperation::Invalid(_) => unreachable!(),
                };
                if !valid_resource(resource) {
                    return Err(error("invalid file resource name"));
                }
                field(key)?;
                Ok(op)
            }),
    )
}

pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, op) in action.file_operations.iter().enumerate() {
            let (kind, resource, key, edge_kind) = match op {
                AilFileOperation::Read { resource, key } => (
                    "read",
                    resource.as_str(),
                    Some(key.as_str()),
                    "file_destination",
                ),
                AilFileOperation::Write { resource, source } => (
                    "write",
                    resource.as_str(),
                    Some(source.as_str()),
                    "file_source",
                ),
                AilFileOperation::ReadBytes { resource, key } => (
                    "read_bytes",
                    resource.as_str(),
                    Some(key.as_str()),
                    "file_destination",
                ),
                AilFileOperation::WriteBytes { resource, source } => (
                    "write_bytes",
                    resource.as_str(),
                    Some(source.as_str()),
                    "file_source",
                ),
                AilFileOperation::Invalid(message) => {
                    ("invalid", message.as_str(), None, "file_destination")
                }
            };
            let node = graph.add_node(
                "FileOperation",
                format!("{}.file{index}", owner.name),
                None,
                attr(&[("file_kind", kind), ("resource", resource)]),
            );
            attach_provenance(graph, &node, format!("action:{}.file:{index}", owner.name));
            if let Some(key) = key {
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
                            format!("action:{}.file:{index}.operand", owner.name),
                        );
                        target
                    });
                graph.add_edge(edge_kind, &node, &target, BTreeMap::new());
            }
            action_order::append(graph, "file_step", &owner, &node, BTreeMap::new(), index);
        }
    }
}

fn unique_node<'a>(core: &'a AilCore, id: &str) -> Result<&'a Node, String> {
    let mut nodes = core.graph.nodes.iter().filter(|n| n.id == id);
    let node = nodes
        .next()
        .ok_or_else(|| error("missing file graph participant"))?;
    if nodes.next().is_some() {
        return Err(error("file graph participants require unique identities"));
    }
    Ok(node)
}
fn participant(core: &AilCore, node: &Node) -> Result<(), String> {
    unique_node(core, &node.id)?;
    let mut seen = BTreeSet::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == "has_provenance")
    {
        if !edge.attributes.is_empty()
            || !seen.insert(&edge.target)
            || unique_node(core, &edge.target)?.kind != "Provenance"
        {
            return Err(error("invalid file participant provenance"));
        }
    }
    Ok(())
}

fn decode_node(core: &AilCore, node: &Node) -> Result<AilFileOperation, String> {
    participant(core, node)?;
    let parents = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect::<Vec<_>>();
    if parents.len() != 1 || parents[0].kind != "file_step" || !valid_step(parents[0]) {
        return Err(error("file operation requires exactly one ordered owner"));
    }
    let owner = unique_node(core, &parents[0].source)?;
    participant(core, owner)?;
    if !executable_action(owner) {
        return Err(error("file operation requires an executable Action owner"));
    }
    if node.kind != "FileOperation"
        || node.type_name.is_some()
        || node
            .attributes
            .keys()
            .map(String::as_str)
            .ne(["file_kind", "resource"])
    {
        return Err(error(
            "file node requires exactly file_kind and resource attributes and no type_name",
        ));
    }
    let resource = &node.attributes["resource"];
    if !valid_resource(resource) {
        return Err(error("invalid file resource name"));
    }
    let edge_kind = match node.attributes["file_kind"].as_str() {
        "read" | "read_bytes" => "file_destination",
        "write" | "write_bytes" => "file_source",
        _ => return Err(error("unknown file_kind")),
    };
    if core.graph.edges.iter().any(|edge| {
        (edge.source == node.id && edge.kind != edge_kind && edge.kind != "has_provenance")
            || (edge.target == node.id && edge.kind != "file_step")
    }) {
        return Err(error("unexpected edge on file operation"));
    }
    let mut operands = core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == edge_kind);
    let edge = operands
        .next()
        .ok_or_else(|| error("file operation has no field operand"))?;
    if operands.next().is_some() || !edge.attributes.is_empty() {
        return Err(error(
            "file operation requires one field edge without attributes",
        ));
    }
    let target = unique_node(core, &edge.target)?;
    let binary = matches!(
        node.attributes["file_kind"].as_str(),
        "read_bytes" | "write_bytes"
    );
    let expected = if binary { "Bytes" } else { "Text" };
    let key = float_operations::scoped_field(core, target, None, expected).map_err(error)?;
    participant(core, target)?;
    for declaration in core
        .graph
        .edges
        .iter()
        .filter(|e| e.kind == "has_field" && e.target == target.id)
    {
        participant(core, unique_node(core, &declaration.source)?)?;
    }
    field(&key)?;
    Ok(match node.attributes["file_kind"].as_str() {
        "read" => AilFileOperation::Read {
            resource: resource.clone(),
            key,
        },
        "write" => AilFileOperation::Write {
            resource: resource.clone(),
            source: key,
        },
        "read_bytes" => AilFileOperation::ReadBytes {
            resource: resource.clone(),
            key,
        },
        "write_bytes" => AilFileOperation::WriteBytes {
            resource: resource.clone(),
            source: key,
        },
        _ => unreachable!(),
    })
}

pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilFileOperation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "file_step")
        .map(|edge| {
            unique_node(core, &edge.target)
                .and_then(|node| decode_node(core, node))
                .unwrap_or_else(AilFileOperation::Invalid)
        })
        .collect()
}

fn executable_action(node: &Node) -> bool {
    node.kind == "Action"
        && node
            .attributes
            .get("kind")
            .is_none_or(|kind| kind != "CompilerPass")
}

fn valid_step(edge: &Edge) -> bool {
    edge.attributes.contains_key("step_index")
        && edge.attributes.iter().all(|(key, value)| {
            matches!(key.as_str(), "step_index" | "operation_index")
                && value.parse::<usize>().is_ok()
        })
}

pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for node in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "FileOperation")
    {
        let result = (|| {
            if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
                return Err(error(
                    "file operations require Application or C interop profile",
                ));
            }
            let owners: Vec<_> = core
                .graph
                .edges
                .iter()
                .filter(|e| e.target == node.id && e.kind == "file_step")
                .collect();
            if owners.len() != 1
                || !core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == owners[0].source && executable_action(n))
            {
                return Err(error(
                    "file operation requires exactly one executable action owner",
                ));
            }
            decode_node(core, node).map(|_| ())
        })();
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-FILE-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = unique_node(core, &edge.source).ok();
        let target = unique_node(core, &edge.target).ok();
        let valid = match edge.kind.as_str() {
            "file_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "FileOperation")
                    && edge
                        .attributes
                        .get("step_index")
                        .is_some_and(|v| v.parse::<usize>().is_ok())
                    && edge.attributes.iter().all(|(k, v)| {
                        matches!(k.as_str(), "step_index" | "operation_index")
                            && v.parse::<usize>().is_ok()
                    })
            }
            "file_source" | "file_destination" => {
                source.is_some_and(|n| n.kind == "FileOperation")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error("AIL-FILE-001", format!("invalid {} file edge", edge.kind))
                    .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}

pub(super) fn namespace(op: &AilFileOperation, alias: &str) -> AilFileOperation {
    let prefix = runtime_subject_key(alias);
    match op {
        AilFileOperation::Read { resource, key } => AilFileOperation::Read {
            resource: format!("{prefix}.{resource}"),
            key: format!("{prefix}.{key}"),
        },
        AilFileOperation::Write { resource, source } => AilFileOperation::Write {
            resource: format!("{prefix}.{resource}"),
            source: format!("{prefix}.{source}"),
        },
        AilFileOperation::ReadBytes { resource, key } => AilFileOperation::ReadBytes {
            resource: format!("{prefix}.{resource}"),
            key: format!("{prefix}.{key}"),
        },
        AilFileOperation::WriteBytes { resource, source } => AilFileOperation::WriteBytes {
            resource: format!("{prefix}.{resource}"),
            source: format!("{prefix}.{source}"),
        },
        AilFileOperation::Invalid(message) => AilFileOperation::Invalid(message.clone()),
    }
}

pub(super) fn instruction(
    op: &AilFileOperation,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let (opcode, resource, operand, key) = match op {
        AilFileOperation::Read { resource, key } => ("FILE_READ_TEXT", resource, "key", key),
        AilFileOperation::Write { resource, source } => {
            ("FILE_WRITE_TEXT", resource, "source", source)
        }
        AilFileOperation::ReadBytes { resource, key } => ("FILE_READ_BYTES", resource, "key", key),
        AilFileOperation::WriteBytes { resource, source } => {
            ("FILE_WRITE_BYTES", resource, "source", source)
        }
        AilFileOperation::Invalid(message) => return Err(error(message)),
    };
    let instruction = AilBytecodeInstruction::new(
        opcode,
        &[("resource", resource.clone()), (operand, key.clone())],
    );
    verify(&instruction)?;
    let expected = if opcode.ends_with("_BYTES") {
        "Bytes"
    } else {
        "Text"
    };
    if runtime_field_type_name(document, key) != Some(expected) {
        return Err(error(format!(
            "file field operand must be a declared {expected} field"
        )));
    }
    Ok(instruction)
}

pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let (keys, field_key) = match instruction.opcode.as_str() {
        "FILE_READ_TEXT" | "FILE_READ_BYTES" => (["key", "resource"], "key"),
        "FILE_WRITE_TEXT" | "FILE_WRITE_BYTES" => (["resource", "source"], "source"),
        _ => return Err(error("unknown file instruction")),
    };
    if instruction.operands.keys().map(String::as_str).ne(keys) {
        return Err(error(
            "file instruction requires exact resource and field operands",
        ));
    }
    if !valid_resource(&instruction.operands["resource"]) {
        return Err(error("invalid file resource name"));
    }
    field(&instruction.operands[field_key])
}

pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let edge_kind = match kind {
        "reads" => "file_source",
        "writes" => "file_destination",
        _ => return Vec::new(),
    };
    let mut result = Vec::new();
    for step in core.graph.edges.iter().filter(|e| e.kind == "file_step") {
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|e| e.source == step.target && e.kind == edge_kind)
        {
            result.push((edge, step.source.as_str()));
        }
    }
    result
}

fn retry_io<T>(mut operation: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    loop {
        match operation() {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}
fn io_error(resource: &str, operation: &str, cause: std::io::Error) -> String {
    format!(
        "AIL-RUNTIME-IO: {operation} failed for file resource {resource}: {}",
        cause.kind()
    )
}
fn open_regular(path: &Path, resource: &str, writing: bool) -> Result<fs::File, String> {
    let mut options = fs::OpenOptions::new();
    if writing {
        options.write(true).create(true).truncate(false);
    } else {
        options.read(true);
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Linux O_NOCTTY | O_NONBLOCK: opening a nonregular file must not acquire a terminal or wait on a FIFO.
        options.custom_flags(0o400 | 0o4000);
    }
    let file = retry_io(|| options.open(path)).map_err(|e| io_error(resource, "open", e))?;
    if !retry_io(|| file.metadata())
        .map_err(|e| io_error(resource, "metadata", e))?
        .is_file()
    {
        return Err(format!(
            "AIL-RUNTIME-IO: file resource {resource} is not a regular file"
        ));
    }
    Ok(file)
}

pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
    allocated: &mut usize,
    resources: &AilFileResources,
) -> Result<(), String> {
    verify(instruction)?;
    let resource = &instruction.operands["resource"];
    let writing = matches!(
        instruction.opcode.as_str(),
        "FILE_WRITE_TEXT" | "FILE_WRITE_BYTES"
    );
    let binary = instruction.opcode.ends_with("_BYTES");
    let grants = if writing {
        &resources.writes
    } else {
        &resources.reads
    };
    let path = grants.get(resource).ok_or_else(|| {
        format!(
            "AIL-RUNTIME-CAPABILITY: no {} grant for file resource {resource}",
            if writing { "write" } else { "read" }
        )
    })?;
    if writing {
        let decoded;
        let bytes = if binary {
            let transport = state
                .get(&instruction.operands["source"])
                .ok_or("AIL-RUNTIME-BYTES: missing byte value")?;
            decoded = bytes_operations::runtime_to_raw(transport)?;
            decoded.as_slice()
        } else {
            text_operations::read_text(state, &instruction.operands["source"])?.as_bytes()
        };
        let mut file = open_regular(path, resource, true)?;
        retry_io(|| file.set_len(0)).map_err(|e| io_error(resource, "truncate", e))?;
        file.write_all(bytes)
            .map_err(|e| io_error(resource, "write", e))?;
    } else {
        let mut file = open_regular(path, resource, false)?;
        let mut bytes = Vec::with_capacity(MAX_TEXT_BYTES + 1);
        let mut buffer = [0u8; 8192];
        while bytes.len() <= MAX_TEXT_BYTES {
            let remaining = (MAX_TEXT_BYTES + 1 - bytes.len()).min(buffer.len());
            let count = retry_io(|| file.read(&mut buffer[..remaining]))
                .map_err(|e| io_error(resource, "read", e))?;
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
        drop(file);
        if bytes.len() > MAX_TEXT_BYTES {
            return Err(if binary {
                "AIL-RUNTIME-BYTES-LIMIT: file bytes exceed 1048576 bytes"
            } else {
                "AIL-RUNTIME-TEXT-LIMIT: file text exceeds 1048576 bytes"
            }
            .into());
        }
        let result = if binary {
            let transport = bytes_operations::runtime_from_raw(&bytes)?;
            bytes_operations::reserve_result(transport.len(), allocated)?;
            transport
        } else {
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| "AIL-RUNTIME-TEXT: file is not valid UTF-8".to_string())?;
            text_operations::reserve_result(text.len(), allocated)?;
            text.to_string()
        };
        state.insert(instruction.operands["key"].clone(), result);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn binary_source_and_protocol_are_explicit() {
        for source in [
            "the system reads bytes from file resource input into data.body",
            "the system writes bytes data.body to file resource output",
        ] {
            let operation = parse_bullet(source)
                .expect("recognized binary file form")
                .unwrap();
            assert_eq!(operation.source(), source);
        }
        verify(&AilBytecodeInstruction::new(
            "FILE_READ_BYTES",
            &[("resource", "input".into()), ("key", "data.body".into())],
        ))
        .unwrap();
    }

    #[test]
    fn binary_files_roundtrip_arbitrary_bytes_and_validate_before_truncating() {
        let files = Files::new();
        let resources = files.resources();
        let raw = (0..=255).collect::<Vec<u8>>();
        fs::write(files.path("input"), &raw).unwrap();
        fs::write(files.path("output"), b"preserved").unwrap();
        let read = AilBytecodeInstruction::new(
            "FILE_READ_BYTES",
            &[("resource", "input".into()), ("key", "data.body".into())],
        );
        let write = AilBytecodeInstruction::new(
            "FILE_WRITE_BYTES",
            &[
                ("resource", "output".into()),
                ("source", "data.body".into()),
            ],
        );
        let mut state = BTreeMap::new();
        let mut allocated = 0;
        execute(&read, &mut state, &mut allocated, &resources).unwrap();
        assert_eq!(allocated, 516);
        execute(&write, &mut state, &mut allocated, &resources).unwrap();
        assert_eq!(fs::read(files.path("output")).unwrap(), raw);
        state.insert("data.body".into(), "hex:FF".into());
        assert!(
            execute(&write, &mut state, &mut allocated, &resources)
                .unwrap_err()
                .contains("BYTES:")
        );
        assert_eq!(fs::read(files.path("output")).unwrap(), raw);
        assert_eq!(allocated, 516);
    }

    #[test]
    fn binary_read_limits_charge_the_list_arena_once_and_preserve_failed_destinations() {
        let files = Files::new();
        let resources = files.resources();
        let mut instruction = read();
        instruction.opcode = "FILE_READ_BYTES".into();
        let mut state = attr(&[("data.text", "preserved")]);
        let mut allocated = 0;
        fs::write(files.path("input"), vec![255; MAX_TEXT_BYTES]).unwrap();
        execute(&instruction, &mut state, &mut allocated, &resources).unwrap();
        assert_eq!(state["data.text"].len(), 2_097_156);
        assert_eq!(allocated, 2_097_156);
        fs::write(files.path("input"), vec![0; MAX_TEXT_BYTES + 1]).unwrap();
        let before = state.clone();
        assert!(
            execute(&instruction, &mut state, &mut allocated, &resources)
                .unwrap_err()
                .contains("BYTES-LIMIT")
        );
        assert_eq!(state, before);
        assert_eq!(allocated, 2_097_156);
        fs::write(files.path("input"), []).unwrap();
        allocated = 67_108_861;
        assert!(
            execute(&instruction, &mut state, &mut allocated, &resources)
                .unwrap_err()
                .contains("BYTES-LIMIT")
        );
        assert_eq!(state, before);
        assert_eq!(allocated, 67_108_861);
        allocated = 67_108_860;
        execute(&instruction, &mut state, &mut allocated, &resources).unwrap();
        assert_eq!(state["data.text"], "hex:");
        assert_eq!(allocated, 67_108_864);
        let mut write = write();
        write.opcode = "FILE_WRITE_BYTES".into();
        state.clear();
        assert!(
            execute(
                &write,
                &mut state,
                &mut allocated,
                &AilFileResources::default()
            )
            .unwrap_err()
            .contains("CAPABILITY")
        );
        assert!(!files.path("output").exists());
    }
    #[test]
    fn binary_core_rejects_storage_identity_and_attached_provenance_forgery() {
        let mut original = core_fixture();
        for node in &mut original.graph.nodes {
            if node.kind == "Field" {
                node.type_name = Some("Bytes".into());
            }
            if node.kind == "FileOperation" {
                let kind = node.attributes["file_kind"].clone();
                node.attributes
                    .insert("file_kind".into(), format!("{kind}_bytes"));
            }
        }
        assert!(check(&original).is_empty(), "{:?}", check(&original));
        for node in original.graph.nodes.iter().filter(|n| {
            matches!(
                n.kind.as_str(),
                "Action" | "Thing" | "Field" | "FileOperation"
            )
        }) {
            let mut bad = original.clone();
            bad.graph.nodes.push(node.clone());
            assert!(!check(&bad).is_empty());
            if let Ok(saved) = parse_ail_core_text(&render_ail_core(&bad)) {
                assert!(!check(&saved).is_empty());
            }
            for edge in original
                .graph
                .edges
                .iter()
                .filter(|e| e.source == node.id && e.kind == "has_provenance")
            {
                let mut bad = original.clone();
                bad.graph.nodes.push(
                    original
                        .graph
                        .nodes
                        .iter()
                        .find(|n| n.id == edge.target)
                        .unwrap()
                        .clone(),
                );
                assert!(!check(&bad).is_empty());
                if let Ok(saved) = parse_ail_core_text(&render_ail_core(&bad)) {
                    assert!(!check(&saved).is_empty());
                }
            }
        }
        let mut bad = original.clone();
        bad.graph.edges.retain(|e| e.kind != "has_field");
        assert!(!check(&bad).is_empty());
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert!(check(&saved).is_empty());
    }

    struct Files {
        root: PathBuf,
    }
    impl Files {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "ail-files-module-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self { root }
        }
        fn path(&self, name: &str) -> PathBuf {
            self.root.join(name)
        }
        fn resources(&self) -> AilFileResources {
            AilFileResources {
                reads: BTreeMap::from([("input".into(), self.path("input"))]),
                writes: BTreeMap::from([("output".into(), self.path("output"))]),
            }
        }
    }
    impl Drop for Files {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    fn read() -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            "FILE_READ_TEXT",
            &[("resource", "input".into()), ("key", "data.text".into())],
        )
    }
    fn write() -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            "FILE_WRITE_TEXT",
            &[
                ("resource", "output".into()),
                ("source", "data.text".into()),
            ],
        )
    }

    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Files manages files.\n\nA Data has:\n- text: Text\n\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Finished\n";
        let mut document = parse_ail_package_spec_text(&package, source).unwrap();
        let action = document.actions.get_mut("Run").unwrap();
        action.file_operations = vec![
            AilFileOperation::Read {
                resource: "input".into(),
                key: "data.text".into(),
            },
            AilFileOperation::Write {
                resource: "output".into(),
                source: "data.text".into(),
            },
        ];
        action.steps.insert(0, AilActionStep::File(1));
        action.steps.insert(0, AilActionStep::File(0));
        elaborate_ail_core(&package, &document)
    }

    #[test]
    fn saved_core_retains_file_effects_and_rejects_extra_or_duplicate_metadata() {
        let original = core_fixture();
        assert_eq!(
            original
                .graph
                .nodes
                .iter()
                .filter(|n| n.kind == "FileOperation")
                .count(),
            2
        );
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert!(check(&saved).is_empty(), "{:?}", check(&saved));
        let action = saved
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "Action")
            .unwrap();
        let operations = from_core(&saved, action);
        assert_eq!(operations.len(), 2);
        assert!(operations.iter().any(
            |op| op.source() == "the system reads text from file resource input into data.text"
        ));
        for kind in ["file_step", "file_destination", "file_source"] {
            let mut candidate = original.clone();
            candidate.graph.edges.push(
                candidate
                    .graph
                    .edges
                    .iter()
                    .find(|e| e.kind == kind)
                    .unwrap()
                    .clone(),
            );
            assert!(
                !check(&parse_ail_core_text(&render_ail_core(&candidate)).unwrap()).is_empty(),
                "{kind}"
            );
            let mut candidate = original.clone();
            candidate
                .graph
                .edges
                .iter_mut()
                .find(|e| e.kind == kind)
                .unwrap()
                .attributes
                .insert("hidden".into(), "ignored".into());
            assert!(!check(&candidate).is_empty(), "{kind} attrs");
        }
        for (attribute, value) in [
            ("file_kind", "append"),
            ("resource", "/tmp/path"),
            ("hidden", "ignored"),
        ] {
            let mut candidate = original.clone();
            candidate
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == "FileOperation")
                .unwrap()
                .attributes
                .insert(attribute.into(), value.into());
            assert!(!check(&candidate).is_empty(), "{attribute}");
        }
    }

    #[test]
    fn core_requires_global_text_executable_owners_and_derives_secret_accesses() {
        let original = core_fixture();
        for profile in ["Compiler", "System", "UI", "AgentTool"] {
            let mut candidate = original.clone();
            candidate.package.profile = profile.into();
            assert!(!check(&candidate).is_empty(), "{profile}");
        }
        let mut compiler = original.clone();
        compiler
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Action")
            .unwrap()
            .attributes
            .insert("kind".into(), "CompilerPass".into());
        assert!(!check(&compiler).is_empty());
        let mut local = original.clone();
        local
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field")
            .unwrap()
            .attributes
            .insert("function_owner".into(), "hidden".into());
        assert!(!check(&local).is_empty());
        let mut wrong_type = original.clone();
        wrong_type
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field")
            .unwrap()
            .type_name = Some("Int".into());
        assert!(!check(&wrong_type).is_empty());
        let mut secret = original;
        secret
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        secret
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field")
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        assert_eq!(accesses(&secret, "reads").len(), 1);
        assert_eq!(accesses(&secret, "writes").len(), 1);
        assert!(
            check_secret_write_protection(&secret)
                .iter()
                .any(|d| d.code == "AIL002")
        );
        assert!(
            check_secret_read_protection(&secret)
                .iter()
                .any(|d| d.code == "AIL005")
        );
    }

    #[test]
    fn parser_namespaces_resource_and_field_without_path_authority() {
        let op = parse_bullet("the system reads text from file resource input into data.text")
            .unwrap()
            .unwrap();
        assert_eq!(
            namespace(&op, "pkg").source(),
            "the system reads text from file resource pkg.input into pkg.data.text"
        );
        let op = parse_bullet("the system writes text data.text to file resource output")
            .unwrap()
            .unwrap();
        assert_eq!(
            namespace(&op, "pkg").source(),
            "the system writes text pkg.data.text to file resource pkg.output"
        );
        for source in [
            "the system reads text from file resource ../input into data.text",
            "the system writes text data.text to file resource /tmp/output",
            "the system reads text from file resource input into invalid",
        ] {
            assert!(parse_bullet(source).unwrap().is_err(), "{source}");
        }
        assert!(parse_bullet("the system records a trace event named Read").is_none());
    }

    #[test]
    fn bindings_require_bounded_static_names_and_nonempty_nul_free_paths() {
        for name in ["input", "pkg.input_1", "_private.A9"] {
            assert!(valid_resource(name), "{name}");
        }
        for name in [
            "", "1input", "a..b", ".input", "input.", "é", "a/b", "a-b", "a b",
        ] {
            assert!(!valid_resource(name), "{name}");
        }
        assert!(valid_resource(&"a".repeat(256)));
        assert!(!valid_resource(&"a".repeat(257)));
        assert_eq!(
            parse_binding("input=some file=part").unwrap(),
            ("input".into(), PathBuf::from("some file=part"))
        );
        for binding in ["input", "input=", "=path", "bad/name=path", "input=a\0b"] {
            assert!(parse_binding(binding).is_err(), "{binding:?}");
        }
    }

    #[test]
    fn read_write_preserve_unicode_nul_and_empty_bytes_and_reopen_each_time() {
        let files = Files::new();
        let resources = files.resources();
        let mut state = BTreeMap::new();
        let mut allocated = 0;
        fs::write(files.path("output"), "long previous content").unwrap();
        fs::write(files.path("input"), "é\0🙂").unwrap();
        execute(&read(), &mut state, &mut allocated, &resources).unwrap();
        state.insert("data.copy".into(), state["data.text"].clone());
        execute(&write(), &mut state, &mut allocated, &resources).unwrap();
        assert_eq!(fs::read(files.path("output")).unwrap(), "é\0🙂".as_bytes());
        assert_eq!(allocated, 7);
        fs::write(files.path("input"), "").unwrap();
        execute(&read(), &mut state, &mut allocated, &resources).unwrap();
        execute(&write(), &mut state, &mut allocated, &resources).unwrap();
        assert!(fs::read(files.path("output")).unwrap().is_empty());
        assert_eq!(state["data.copy"], "é\0🙂");
        assert_eq!(allocated, 8);
    }

    #[test]
    fn grant_precedes_state_or_io_and_invalid_source_cannot_truncate() {
        let files = Files::new();
        let resources = files.resources();
        let mut state = BTreeMap::from([
            ("input".into(), files.path("input").display().to_string()),
            ("output".into(), files.path("output").display().to_string()),
        ]);
        let mut allocated = 0;
        for instruction in [read(), write()] {
            assert!(
                execute(
                    &instruction,
                    &mut state,
                    &mut allocated,
                    &AilFileResources::default()
                )
                .unwrap_err()
                .starts_with("AIL-RUNTIME-CAPABILITY")
            );
        }
        fs::write(files.path("output"), "original").unwrap();
        assert!(
            execute(&write(), &mut state, &mut allocated, &resources)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT:")
        );
        assert_eq!(fs::read(files.path("output")).unwrap(), b"original");
        state.insert("data.text".into(), "x".repeat(MAX_TEXT_BYTES + 1));
        assert!(
            execute(&write(), &mut state, &mut allocated, &resources)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT-LIMIT")
        );
        assert_eq!(fs::read(files.path("output")).unwrap(), b"original");
    }

    #[test]
    fn reads_validate_size_then_utf8_then_budget_without_mutating_on_error() {
        let files = Files::new();
        let resources = files.resources();
        let mut state = BTreeMap::from([("data.text".into(), "original".into())]);
        let mut allocated = 0;
        for (bytes, expected) in [
            (vec![b'x'; MAX_TEXT_BYTES + 1], "AIL-RUNTIME-TEXT-LIMIT"),
            (vec![0xff; MAX_TEXT_BYTES + 1], "AIL-RUNTIME-TEXT-LIMIT"),
            (vec![0xff], "AIL-RUNTIME-TEXT:"),
        ] {
            fs::write(files.path("input"), bytes).unwrap();
            assert!(
                execute(&read(), &mut state, &mut allocated, &resources)
                    .unwrap_err()
                    .starts_with(expected)
            );
            assert_eq!(state["data.text"], "original");
            assert_eq!(allocated, 0);
        }
        fs::write(files.path("input"), vec![b'x'; MAX_TEXT_BYTES]).unwrap();
        execute(&read(), &mut state, &mut allocated, &resources).unwrap();
        assert_eq!(state["data.text"].len(), MAX_TEXT_BYTES);
        assert_eq!(allocated, MAX_TEXT_BYTES);
        allocated = 64 * 1024 * 1024;
        fs::write(files.path("input"), "").unwrap();
        assert!(
            execute(&read(), &mut state, &mut allocated, &resources)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT-LIMIT")
        );
        assert_eq!(state["data.text"].len(), MAX_TEXT_BYTES);
        assert_eq!(allocated, 64 * 1024 * 1024);
    }

    #[test]
    fn file_types_and_missing_parent_return_io_errors() {
        let files = Files::new();
        let mut resources = files.resources();
        let mut state = BTreeMap::from([("data.text".into(), "hello".into())]);
        let mut allocated = 0;
        for path in [files.root.clone(), files.path("missing/child")] {
            resources.reads.insert("input".into(), path.clone());
            resources.writes.insert("output".into(), path);
            for instruction in [read(), write()] {
                assert!(
                    execute(&instruction, &mut state, &mut allocated, &resources)
                        .unwrap_err()
                        .starts_with("AIL-RUNTIME-IO")
                );
            }
        }
    }

    #[test]
    fn bytecode_protocol_rejects_hidden_operands_and_dynamic_paths() {
        for mut instruction in [read(), write()] {
            assert!(verify(&instruction).is_ok());
            instruction
                .operands
                .insert("path".into(), "/tmp/hidden".into());
            assert!(verify(&instruction).is_err());
            instruction.operands.remove("path");
            instruction
                .operands
                .insert("resource".into(), "../input".into());
            assert!(verify(&instruction).is_err());
        }
    }

    #[test]
    fn reverse_grants_cannot_authorize_operations_or_create_files() {
        let files = Files::new();
        let resources = AilFileResources {
            reads: BTreeMap::from([("output".into(), files.path("output"))]),
            writes: BTreeMap::from([("input".into(), files.path("input"))]),
        };
        let mut state = BTreeMap::from([("data.text".into(), "hello".into())]);
        let mut allocated = 0;
        for instruction in [read(), write()] {
            assert!(
                execute(&instruction, &mut state, &mut allocated, &resources)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-CAPABILITY")
            );
        }
        assert!(!files.path("output").exists());
        assert!(!files.path("input").exists());
        assert_eq!(allocated, 0);
    }

    #[cfg(unix)]
    #[test]
    fn bound_symlinks_follow_host_paths_and_socket_targets_are_rejected() {
        use std::os::unix::{fs::symlink, net::UnixListener};
        let files = Files::new();
        let resources = files.resources();
        fs::write(files.path("actual-input"), "from symlink").unwrap();
        fs::write(files.path("actual-output"), "previous longer content").unwrap();
        symlink(files.path("actual-input"), files.path("input")).unwrap();
        symlink(files.path("actual-output"), files.path("output")).unwrap();
        let mut state = BTreeMap::new();
        let mut allocated = 0;
        execute(&read(), &mut state, &mut allocated, &resources).unwrap();
        execute(&write(), &mut state, &mut allocated, &resources).unwrap();
        assert_eq!(
            fs::read(files.path("actual-output")).unwrap(),
            b"from symlink"
        );
        fs::remove_file(files.path("input")).unwrap();
        let _socket = UnixListener::bind(files.path("input")).unwrap();
        assert!(
            execute(&read(), &mut state, &mut allocated, &resources)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-IO")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn fifos_and_devices_fail_without_blocking_or_consuming_budget() {
        use std::os::unix::ffi::OsStrExt;
        unsafe extern "C" {
            fn mkfifo(path: *const std::ffi::c_char, mode: u32) -> i32;
        }
        let files = Files::new();
        let fifo = files.path("fifo");
        let c_path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        // The C string is live and NUL-terminated for this libc call.
        assert_eq!(unsafe { mkfifo(c_path.as_ptr(), 0o600) }, 0);
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut state = BTreeMap::from([("data.text".into(), "unchanged".into())]);
            let mut allocated = 0;
            let mut errors = Vec::new();
            for path in [fifo, PathBuf::from("/dev/null")] {
                let resources = AilFileResources {
                    reads: BTreeMap::from([("input".into(), path.clone())]),
                    writes: BTreeMap::from([("output".into(), path)]),
                };
                for instruction in [read(), write()] {
                    errors.push(execute(
                        &instruction,
                        &mut state,
                        &mut allocated,
                        &resources,
                    ));
                }
            }
            let _ = sender.send((errors, state, allocated));
        });
        let (errors, state, allocated) = receiver
            .recv_timeout(std::time::Duration::from_secs(3))
            .expect("file type rejection must not block on a FIFO");
        assert!(
            errors
                .iter()
                .all(|result| result.as_ref().unwrap_err().starts_with("AIL-RUNTIME-IO"))
        );
        assert_eq!(state["data.text"], "unchanged");
        assert_eq!(allocated, 0);
    }
}
