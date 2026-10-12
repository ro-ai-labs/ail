//! Explicit caller-bound process effects with typed, atomic results.
use super::*;
use crate::expression::IntExpr;

const MAX_TEXT_BYTES: usize = 1_048_576;
const MAX_ARGUMENTS: usize = 256;
const MAX_ARGUMENT_BYTES: usize = 65_536;
const ROLES: [(&str, &str, &str); 6] = [
    ("arguments", "process_arguments", "List<Text>"),
    ("input", "process_input", "Text"),
    ("timeout_ms", "process_timeout", "Int"),
    ("stdout", "process_stdout", "Text"),
    ("stderr", "process_stderr", "Text"),
    ("status", "process_status", "Int"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilProcessOperation {
    Run {
        resource: String,
        arguments: String,
        input: String,
        timeout_ms: String,
        stdout: String,
        stderr: String,
        status: String,
    },
    Invalid(String),
}

#[derive(Default, Clone, Debug)]
pub struct AilProcessResources {
    pub executables: BTreeMap<String, PathBuf>,
}

impl AilProcessOperation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Run {
                resource,
                arguments,
                input,
                timeout_ms,
                stdout,
                stderr,
                status,
            } => {
                format!(
                    "the system runs process resource {resource} with arguments {arguments} and input {input} within {timeout_ms} milliseconds into {stdout} with errors {stderr} and status {status}"
                )
            }
            Self::Invalid(message) => format!("invalid process operation: {message}"),
        }
    }
}
fn error(message: impl Display) -> String {
    format!("AIL-PROCESS-001: {message}")
}
fn runtime_error(message: &str) -> String {
    format!("AIL-RUNTIME-PROCESS: {message}")
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn io_error() -> String {
    "AIL-RUNTIME-IO: process host operation failed".into()
}
fn field(key: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(key), Ok(IntExpr::Field(parsed)) if parsed == key) {
        Ok(())
    } else {
        Err(error("process operand must be a field reference"))
    }
}
/// Whether a process resource is a bounded, optionally qualified ASCII identifier.
pub fn valid_resource(name: &str) -> bool {
    file_operations::valid_resource(name)
}
fn valid_path(path: &Path) -> bool {
    let bytes = path.as_os_str().as_encoded_bytes();
    path.is_absolute() && !bytes.is_empty() && bytes.len() <= 4095 && !bytes.contains(&0)
}
/// Parse the resource=absolute-path part of an explicit executable grant.
pub fn parse_binding(binding: &str) -> Result<(String, PathBuf), String> {
    let (resource, path) = binding
        .split_once('=')
        .ok_or("AIL-RUNTIME-CAPABILITY: process binding requires resource=absolute-path")?;
    if !valid_resource(resource) || !valid_path(Path::new(path)) {
        return Err("AIL-RUNTIME-CAPABILITY: invalid process resource binding".into());
    }
    Ok((resource.into(), path.into()))
}
/// Check every host binding, including unused entries, without opening paths.
pub(super) fn validate_resources(resources: &AilProcessResources) -> Result<(), String> {
    for (resource, path) in &resources.executables {
        if !valid_resource(resource) || !valid_path(path) {
            return Err("AIL-RUNTIME-CAPABILITY: invalid process resource binding".into());
        }
    }
    Ok(())
}
pub(super) fn recognized_bullet(text: &str) -> bool {
    let text = trim_sentence(text);
    text.strip_prefix("the system ")
        .or_else(|| text.strip_prefix("the function "))
        .and_then(|rest| rest.strip_prefix("runs process"))
        .is_some_and(|rest| {
            rest.chars()
                .next()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_')
        })
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilProcessOperation, String>> {
    let text = trim_sentence(text);
    if !text.starts_with("the system ") || !recognized_bullet(&text) {
        return None;
    }
    Some((|| {
        let rest = text
            .strip_prefix("the system runs process resource ")
            .ok_or_else(|| error("malformed process operation"))?;
        fn split<'a>(value: &'a str, separator: &str) -> Result<(&'a str, &'a str), String> {
            value
                .split_once(separator)
                .ok_or_else(|| error("malformed process operation"))
        }
        let (resource, rest) = split(rest, " with arguments ")?;
        let (arguments, rest) = split(rest, " and input ")?;
        let (input, rest) = split(rest, " within ")?;
        let (timeout_ms, rest) = split(rest, " milliseconds into ")?;
        let (stdout, rest) = split(rest, " with errors ")?;
        let (stderr, status) = split(rest, " and status ")?;
        let op = AilProcessOperation::Run {
            resource: resource.into(),
            arguments: arguments.into(),
            input: input.into(),
            timeout_ms: timeout_ms.into(),
            stdout: stdout.into(),
            stderr: stderr.into(),
            status: status.into(),
        };
        verify(&raw_instruction(&op)?)?;
        Ok(op)
    })())
}
fn components(op: &AilProcessOperation) -> Result<(&str, [&str; 6]), String> {
    match op {
        AilProcessOperation::Run {
            resource,
            arguments,
            input,
            timeout_ms,
            stdout,
            stderr,
            status,
        } => Ok((
            resource,
            [arguments, input, timeout_ms, stdout, stderr, status],
        )),
        AilProcessOperation::Invalid(message) => Err(error(message)),
    }
}
fn raw_instruction(op: &AilProcessOperation) -> Result<AilBytecodeInstruction, String> {
    let (resource, fields) = components(op)?;
    let mut operands = BTreeMap::from([("resource".into(), resource.into())]);
    for ((name, _, _), value) in ROLES.iter().zip(fields) {
        operands.insert((*name).into(), value.into());
    }
    Ok(AilBytecodeInstruction {
        opcode: "PROCESS_RUN".into(),
        operands,
    })
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, op) in action.process_operations.iter().enumerate() {
            let parts = components(op);
            let resource = parts.as_ref().map(|parts| parts.0).unwrap_or("");
            let node = graph.add_node(
                "ProcessOperation",
                format!("{}.process{index}", owner.name),
                None,
                attr(&[
                    (
                        "process_kind",
                        if parts.is_ok() { "run" } else { "invalid" },
                    ),
                    ("resource", resource),
                ]),
            );
            attach_provenance(
                graph,
                &node,
                format!("action:{}.process:{index}", owner.name),
            );
            if let Ok((_, fields)) = parts {
                for ((_, edge_kind, _), key) in ROLES.iter().zip(fields) {
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
                                format!("action:{}.process:{index}.{edge_kind}", owner.name),
                            );
                            target
                        });
                    graph.add_edge(*edge_kind, &node, &target, BTreeMap::new());
                }
            }
            action_order::append(graph, "process_step", &owner, &node, BTreeMap::new(), index);
        }
    }
}
fn executable_action(node: &Node) -> bool {
    node.kind == "Action"
        && node
            .attributes
            .get("kind")
            .is_none_or(|kind| kind != "CompilerPass")
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
    let mut matches = core.graph.nodes.iter().filter(|node| node.id == id);
    let node = matches
        .next()
        .ok_or_else(|| error("missing process graph node"))?;
    if matches.next().is_some() {
        return Err(error("process graph node identity must be unique"));
    }
    Ok(node)
}
fn decode_node(core: &AilCore, node: &Node) -> Result<AilProcessOperation, String> {
    unique_node(core, &node.id)?;
    if node.kind != "ProcessOperation"
        || node.type_name.is_some()
        || node
            .attributes
            .keys()
            .map(String::as_str)
            .ne(["process_kind", "resource"])
        || node.attributes["process_kind"] != "run"
    {
        return Err(error(
            "process node requires exactly process_kind=run and resource, without a type annotation",
        ));
    }
    if !valid_resource(&node.attributes["resource"]) {
        return Err(error("invalid process resource name"));
    }
    let owners = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect::<Vec<_>>();
    if owners.len() != 1
        || owners[0].kind != "process_step"
        || !valid_step(owners[0])
        || !executable_action(unique_node(core, &owners[0].source)?)
    {
        return Err(error(
            "process operation requires exactly one ordered executable-action owner",
        ));
    }
    let mut fields = BTreeMap::new();
    for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
        if !edge.attributes.is_empty() {
            return Err(error(
                "process operand and provenance edges require no attributes",
            ));
        }
        let target = unique_node(core, &edge.target)?;
        if edge.kind == "has_provenance" && target.kind == "Provenance" {
            continue;
        }
        let (name, _, kind) = ROLES
            .iter()
            .find(|(_, kind, _)| *kind == edge.kind)
            .ok_or_else(|| error("unexpected process operand edge"))?;
        let key = float_operations::scoped_field(core, target, None, kind).map_err(error)?;
        for declaration in core
            .graph
            .edges
            .iter()
            .filter(|e| e.kind == "has_field" && e.target == target.id)
        {
            unique_node(core, &declaration.source)?;
        }
        if fields.insert(*name, key).is_some() {
            return Err(error("duplicate process operand edge"));
        }
    }
    if fields.len() != ROLES.len() {
        return Err(error("process operation requires all six field operands"));
    }
    let op = AilProcessOperation::Run {
        resource: node.attributes["resource"].clone(),
        arguments: fields["arguments"].clone(),
        input: fields["input"].clone(),
        timeout_ms: fields["timeout_ms"].clone(),
        stdout: fields["stdout"].clone(),
        stderr: fields["stderr"].clone(),
        status: fields["status"].clone(),
    };
    verify(&raw_instruction(&op)?)?;
    Ok(op)
}
pub(super) fn from_core(core: &AilCore, owner: &Node) -> Vec<AilProcessOperation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == owner.id && e.kind == "process_step")
        .map(|edge| {
            core.graph
                .nodes
                .iter()
                .find(|n| n.id == edge.target)
                .ok_or_else(|| error("missing process operation node"))
                .and_then(|node| decode_node(core, node))
                .unwrap_or_else(AilProcessOperation::Invalid)
        })
        .collect()
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for node in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "ProcessOperation")
    {
        let result = if matches!(core.package.profile.as_str(), "Application" | "C interop") {
            decode_node(core, node).map(|_| ())
        } else {
            Err(error(
                "process operations require Application or C interop profile",
            ))
        };
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-PROCESS-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = core.graph.nodes.iter().find(|n| n.id == edge.source);
        let target = core.graph.nodes.iter().find(|n| n.id == edge.target);
        let valid = if edge.kind == "process_step" {
            source.is_some_and(executable_action)
                && target.is_some_and(|n| n.kind == "ProcessOperation")
                && valid_step(edge)
        } else if ROLES.iter().any(|(_, kind, _)| *kind == edge.kind) {
            source.is_some_and(|n| n.kind == "ProcessOperation")
                && target.is_some_and(|n| n.kind == "Field")
                && edge.attributes.is_empty()
        } else {
            true
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-PROCESS-001",
                    format!("invalid {} process edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace(op: &AilProcessOperation, alias: &str) -> AilProcessOperation {
    let qualify = |value: &str| format!("{}.{value}", runtime_subject_key(alias));
    match op {
        AilProcessOperation::Run {
            resource,
            arguments,
            input,
            timeout_ms,
            stdout,
            stderr,
            status,
        } => AilProcessOperation::Run {
            resource: qualify(resource),
            arguments: qualify(arguments),
            input: qualify(input),
            timeout_ms: qualify(timeout_ms),
            stdout: qualify(stdout),
            stderr: qualify(stderr),
            status: qualify(status),
        },
        AilProcessOperation::Invalid(message) => AilProcessOperation::Invalid(message.clone()),
    }
}
pub(super) fn instruction(
    op: &AilProcessOperation,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let instruction = raw_instruction(op)?;
    verify(&instruction)?;
    for (name, _, kind) in ROLES {
        if runtime_field_type_name(document, &instruction.operands[name]) != Some(kind) {
            return Err(error(format!(
                "process {name} must be a declared global {kind} field"
            )));
        }
    }
    Ok(instruction)
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    if instruction.opcode != "PROCESS_RUN"
        || instruction.operands.keys().map(String::as_str).ne([
            "arguments",
            "input",
            "resource",
            "status",
            "stderr",
            "stdout",
            "timeout_ms",
        ])
    {
        return Err(error(
            "PROCESS_RUN requires exactly resource, arguments, input, timeout_ms, stdout, stderr and status",
        ));
    }
    if !valid_resource(&instruction.operands["resource"]) {
        return Err(error("invalid process resource name"));
    }
    for (name, _, _) in ROLES {
        field(&instruction.operands[name])?;
    }
    let destinations = [
        &instruction.operands["stdout"],
        &instruction.operands["stderr"],
        &instruction.operands["status"],
    ];
    if destinations[0] == destinations[1]
        || destinations[0] == destinations[2]
        || destinations[1] == destinations[2]
    {
        return Err(error("process destinations must be distinct"));
    }
    Ok(())
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let roles = match kind {
        "reads" => &ROLES[..3],
        "writes" => &ROLES[3..],
        _ => return Vec::new(),
    };
    let mut result = Vec::new();
    for step in core.graph.edges.iter().filter(|e| e.kind == "process_step") {
        for edge in
            core.graph.edges.iter().filter(|e| {
                e.source == step.target && roles.iter().any(|(_, role, _)| *role == e.kind)
            })
        {
            result.push((edge, step.source.as_str()));
        }
    }
    result
}
fn argument_bytes(path: &Path, arguments: &[String]) -> Result<usize, String> {
    let total = arguments.iter().try_fold(
        path.as_os_str().as_encoded_bytes().len() + 1,
        |total, argument| total.checked_add(argument.len() + 1),
    );
    if arguments.len() > MAX_ARGUMENTS || total.is_none_or(|total| total > MAX_ARGUMENT_BYTES) {
        return Err("AIL-RUNTIME-PROCESS-LIMIT: process arguments exceed limits".into());
    }
    if arguments.iter().any(|argument| argument.contains('\0')) {
        return Err(runtime_error("process argument contains NUL"));
    }
    Ok(total.unwrap())
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn invoke(
    path: &Path,
    arguments: &[String],
    input: &str,
    timeout_ms: i64,
) -> Result<(Vec<u8>, Vec<u8>, i64), String> {
    use crate::process_host::{ProcessRequest, ProcessResult, ail_process_run};
    let mut packed = Vec::new();
    packed
        .try_reserve_exact(argument_bytes(path, arguments)?)
        .map_err(|_| io_error())?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(arguments.len() + 1)
        .map_err(|_| io_error())?;
    for value in std::iter::once(path.as_os_str().as_encoded_bytes())
        .chain(arguments.iter().map(|value| value.as_bytes()))
    {
        offsets.push(packed.len());
        packed.extend_from_slice(value);
        packed.push(0);
    }
    let mut argv = Vec::new();
    argv.try_reserve_exact(offsets.len() + 1)
        .map_err(|_| io_error())?;
    argv.extend(
        offsets
            .into_iter()
            .map(|offset| packed.as_ptr().wrapping_add(offset)),
    );
    argv.push(std::ptr::null());
    let buffer = || -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(MAX_TEXT_BYTES + 1)
            .map_err(|_| io_error())?;
        bytes.resize(MAX_TEXT_BYTES + 1, 0);
        Ok(bytes)
    };
    let mut stdout = buffer()?;
    let mut stderr = buffer()?;
    let request = ProcessRequest {
        executable: packed.as_ptr(),
        argv: argv.as_ptr(),
        input: input.as_ptr(),
        input_len: input.len() as u64,
        timeout_ms: timeout_ms as u64,
        stdout: stdout.as_mut_ptr(),
        stderr: stderr.as_mut_ptr(),
        capacity: (MAX_TEXT_BYTES + 1) as u64,
    };
    let mut result = ProcessResult {
        stdout_len: 0,
        stderr_len: 0,
        status: 0,
    };
    // All C strings, argv pointers and writable capture buffers remain live for
    // the synchronous call. Only the engine owns its child and descriptors.
    let status = unsafe { ail_process_run(&request, &mut result) };
    match status {
        0 => {}
        1 => return Err(runtime_error("process setup or execution failed")),
        2 => return Err("AIL-RUNTIME-TIMEOUT: process deadline expired".into()),
        4 => return Err("AIL-RUNTIME-TEXT-LIMIT: process output exceeds 1048576 bytes".into()),
        _ => return Err(io_error()),
    }
    if result.stdout_len > MAX_TEXT_BYTES as u64 || result.stderr_len > MAX_TEXT_BYTES as u64 {
        return Err("AIL-RUNTIME-TEXT-LIMIT: process output exceeds 1048576 bytes".into());
    }
    stdout.truncate(result.stdout_len as usize);
    stderr.truncate(result.stderr_len as usize);
    Ok((stdout, stderr, result.status))
}
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn invoke(_: &Path, _: &[String], _: &str, _: i64) -> Result<(Vec<u8>, Vec<u8>, i64), String> {
    Err("AIL-RUNTIME-CAPABILITY: process execution requires Linux x86-64".into())
}
pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
    allocated: &mut usize,
    resources: &AilProcessResources,
) -> Result<(), String> {
    verify(instruction)?;
    let operands = &instruction.operands;
    let path = resources
        .executables
        .get(&operands["resource"])
        .ok_or("AIL-RUNTIME-CAPABILITY: process resource is not granted")?;
    let raw_arguments = state
        .get(&operands["arguments"])
        .ok_or("AIL-RUNTIME-LIST: missing process arguments field")?;
    let arguments = list_operations::parse_text_items(raw_arguments)?;
    argument_bytes(path, &arguments)?;
    let input = text_operations::read_text(state, &operands["input"])?;
    let timeout = state
        .get(&operands["timeout_ms"])
        .ok_or("AIL-RUNTIME-INT: missing process timeout field")?;
    let timeout = crate::expression::parse_integer(timeout)?;
    if !(1..=60_000).contains(&timeout) {
        return Err(runtime_error(
            "process timeout must be 1 through 60000 milliseconds",
        ));
    }
    let (stdout, stderr, status) = invoke(path, &arguments, input, timeout)?;
    let stdout = String::from_utf8(stdout)
        .map_err(|_| "AIL-RUNTIME-TEXT: process stdout is not valid UTF-8")?;
    let stderr = String::from_utf8(stderr)
        .map_err(|_| "AIL-RUNTIME-TEXT: process stderr is not valid UTF-8")?;
    let mut next_allocated = *allocated;
    text_operations::reserve_result(stdout.len(), &mut next_allocated)?;
    text_operations::reserve_result(stderr.len(), &mut next_allocated)?;
    let status = status.to_string();
    // Inputs and both results are complete before any destination or budget is
    // changed. Inputs may alias outputs; existing aliases retain their storage.
    state.insert(operands["stdout"].clone(), stdout);
    state.insert(operands["stderr"].clone(), stderr);
    state.insert(operands["status"].clone(), status);
    *allocated = next_allocated;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const BULLET: &str = "the system runs process resource tool with arguments data.arguments and input data.input within data.timeout milliseconds into data.output with errors data.errors and status data.status";
    fn operation() -> AilProcessOperation {
        AilProcessOperation::Run {
            resource: "tool".into(),
            arguments: "data.arguments".into(),
            input: "data.input".into(),
            timeout_ms: "data.timeout".into(),
            stdout: "data.output".into(),
            stderr: "data.errors".into(),
            status: "data.status".into(),
        }
    }
    fn command() -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            "PROCESS_RUN",
            &[
                ("resource", "tool".into()),
                ("arguments", "data.arguments".into()),
                ("input", "data.input".into()),
                ("timeout_ms", "data.timeout".into()),
                ("stdout", "data.output".into()),
                ("stderr", "data.errors".into()),
                ("status", "data.status".into()),
            ],
        )
    }
    fn grants() -> AilProcessResources {
        AilProcessResources {
            executables: BTreeMap::from([("tool".into(), PathBuf::from("/bin/cat"))]),
        }
    }
    fn state() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("data.arguments".into(), "[]".into()),
            ("data.input".into(), "input λ\0\n".into()),
            ("data.timeout".into(), "1000".into()),
            ("data.output".into(), "old output".into()),
            ("data.errors".into(), "old errors".into()),
            ("data.status".into(), "27".into()),
        ])
    }
    fn core() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = format!(
            "The application Process manages tools.\n\nA Data has:\n- arguments: List<Text>\n- input: Text\n- timeout: Int\n- output: Text\n- errors: Text\n- status: Int\n\nAction: Run.\nWhen Run happens:\n- {BULLET}\n- the system records a trace event named Done\n"
        );
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, &source).unwrap(),
        )
    }
    #[test]
    fn exact_source_roundtrips_and_namespaces_resource_and_fields() {
        assert_eq!(parse_bullet(BULLET).unwrap().unwrap(), operation());
        assert_eq!(operation().source(), BULLET);
        assert_eq!(
            parse_bullet(&format!("{BULLET}. ")).unwrap().unwrap(),
            operation()
        );
        assert_eq!(
            namespace(&operation(), "Child").source(),
            BULLET
                .replace("resource tool", "resource child.tool")
                .replace("data.", "child.data.")
        );
    }
    #[test]
    fn malformed_process_syntax_is_recognized_in_both_body_subjects() {
        for subject in ["the system", "the function"] {
            for rest in [
                "runs process",
                "runs process:",
                "runs process resource",
                "runs process resource tool with arguments data.args",
            ] {
                assert!(recognized_bullet(&format!("{subject} {rest}")), "{rest}");
            }
        }
        for text in [
            "the system runs processing",
            "the system runs processional work",
            "the system writes text data.input to standard output",
        ] {
            assert!(!recognized_bullet(text));
            assert!(parse_bullet(text).is_none());
        }
    }
    #[test]
    fn binding_validation_is_absolute_bounded_and_checks_unused_grants() {
        assert_eq!(
            parse_binding("tool=/bin/cat").unwrap(),
            ("tool".into(), PathBuf::from("/bin/cat"))
        );
        let exact = format!("/{}", "x".repeat(4094));
        assert!(parse_binding(&format!("tool={exact}")).is_ok());
        for input in [
            "tool=cat".to_string(),
            "tool=".into(),
            "bad name=/bin/cat".into(),
            "tool=/bin/\0cat".into(),
            format!("tool={exact}x"),
        ] {
            assert!(
                parse_binding(&input)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-CAPABILITY:")
            );
        }
        let mut resources = grants();
        resources
            .executables
            .insert("unused".into(), "relative".into());
        assert!(
            validate_resources(&resources)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-CAPABILITY:")
        );
    }
    #[test]
    fn protocol_requires_exact_fields_and_three_distinct_destinations() {
        verify(&command()).unwrap();
        let mut changed = command();
        changed.operands.insert("grant".into(), "true".into());
        assert!(verify(&changed).is_err());
        for key in [
            "arguments",
            "input",
            "timeout_ms",
            "stdout",
            "stderr",
            "status",
        ] {
            let mut changed = command();
            changed.operands.insert(key.into(), "data.value + 1".into());
            assert!(verify(&changed).is_err(), "{key}");
        }
        for key in ["stderr", "status"] {
            let mut changed = command();
            changed.operands.insert(key.into(), "data.output".into());
            assert!(verify(&changed).is_err());
        }
        let mut aliases = command();
        aliases
            .operands
            .insert("stdout".into(), "data.input".into());
        aliases
            .operands
            .insert("status".into(), "data.timeout".into());
        verify(&aliases).unwrap();
    }
    #[test]
    fn authority_and_typed_input_errors_precede_host_execution_atomically() {
        let mut values = BTreeMap::new();
        let mut allocated = 123;
        assert!(
            execute(
                &command(),
                &mut values,
                &mut allocated,
                &AilProcessResources::default()
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-CAPABILITY:")
        );
        assert!(
            execute(&command(), &mut values, &mut allocated, &grants())
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST:")
        );
        values.insert("data.arguments".into(), "[]".into());
        assert!(
            execute(&command(), &mut values, &mut allocated, &grants())
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT:")
        );
        values.insert("data.input".into(), "".into());
        assert!(
            execute(&command(), &mut values, &mut allocated, &grants())
                .unwrap_err()
                .starts_with("AIL-RUNTIME-INT:")
        );
        for timeout in ["0", "60001", "-1"] {
            values.insert("data.timeout".into(), timeout.into());
            let before = values.clone();
            assert!(
                execute(&command(), &mut values, &mut allocated, &grants())
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-PROCESS:")
            );
            assert_eq!(values, before);
            assert_eq!(allocated, 123);
        }
    }
    #[test]
    fn complete_list_validation_precedes_process_bounds_and_nul_rejection() {
        let mut values = state();
        let mut allocated = 3;
        for (arguments, expected) in [
            ("[\"\\u0000\",false]".into(), "AIL-RUNTIME-LIST:"),
            (
                format!("[{}]", vec!["\"\""; 257].join(",")),
                "AIL-RUNTIME-PROCESS-LIMIT:",
            ),
            (
                format!("[\"{}\"]", "x".repeat(65536)),
                "AIL-RUNTIME-PROCESS-LIMIT:",
            ),
            ("[\"\\u0000\"]".into(), "AIL-RUNTIME-PROCESS:"),
        ] {
            values.insert("data.arguments".into(), arguments);
            let before = values.clone();
            assert!(
                execute(&command(), &mut values, &mut allocated, &grants())
                    .unwrap_err()
                    .starts_with(expected)
            );
            assert_eq!(values, before);
            assert_eq!(allocated, 3);
        }
    }
    #[test]
    fn core_has_explicit_roles_and_preserves_source_and_saved_order() {
        let original = core();
        assert_eq!(
            original
                .graph
                .nodes
                .iter()
                .filter(|n| n.kind == "ProcessOperation")
                .count(),
            1
        );
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let rendered = elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, &render_ail_spec_from_core(&original)).unwrap(),
        );
        for checked in [
            original.clone(),
            parse_ail_core_text(&render_ail_core(&original)).unwrap(),
            rendered,
        ] {
            assert!(
                check_ail_core(&checked).is_empty(),
                "{:?}",
                check_ail_core(&checked)
            );
            let program = compile_ail_core_bytecode(&checked).unwrap();
            assert!(
                program.actions["Run"]
                    .instructions
                    .iter()
                    .any(|i| i == &command())
            );
            assert!(render_ail_spec_from_core(&checked).contains(BULLET));
        }
    }
    fn rejected_direct_and_saved(candidate: &AilCore) {
        assert!(!check(candidate).is_empty());
        assert!(compile_ail_core_bytecode(candidate).is_err());
        if let Ok(saved) = parse_ail_core_text(&render_ail_core(candidate)) {
            assert!(!check(&saved).is_empty());
            assert!(compile_ail_core_bytecode(&saved).is_err());
        }
    }
    #[test]
    fn core_rejects_duplicate_missing_and_unexpected_operand_metadata() {
        let original = core();
        for kind in std::iter::once("process_step").chain(ROLES.iter().map(|(_, edge, _)| *edge)) {
            let mut duplicate = original.clone();
            duplicate.graph.edges.push(
                duplicate
                    .graph
                    .edges
                    .iter()
                    .find(|e| e.kind == kind)
                    .unwrap()
                    .clone(),
            );
            rejected_direct_and_saved(&duplicate);
            let mut extra = original.clone();
            extra
                .graph
                .edges
                .iter_mut()
                .find(|e| e.kind == kind)
                .unwrap()
                .attributes
                .insert("hidden".into(), "ignored".into());
            rejected_direct_and_saved(&extra);
            let mut missing = original.clone();
            missing.graph.edges.retain(|e| e.kind != kind);
            rejected_direct_and_saved(&missing);
        }
        for (key, value) in [
            ("process_kind", "spawn"),
            ("resource", "bad name"),
            ("function_owner", "Hidden"),
        ] {
            let mut changed = original.clone();
            changed
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == "ProcessOperation")
                .unwrap()
                .attributes
                .insert(key.into(), value.into());
            rejected_direct_and_saved(&changed);
        }
        let mut typed = original.clone();
        typed
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "ProcessOperation")
            .unwrap()
            .type_name = Some("Text".into());
        rejected_direct_and_saved(&typed);
        let mut hidden = original.clone();
        let owner = hidden.graph.find_node("Action", "Run").unwrap().clone();
        let op = hidden
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "ProcessOperation")
            .unwrap()
            .clone();
        hidden.graph.add_edge("reads", &owner, &op, BTreeMap::new());
        rejected_direct_and_saved(&hidden);
        let mut alias = original;
        let stdout = alias
            .graph
            .edges
            .iter()
            .find(|e| e.kind == "process_stdout")
            .unwrap()
            .target
            .clone();
        alias
            .graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "process_stderr")
            .unwrap()
            .target = stdout;
        rejected_direct_and_saved(&alias);
    }
    #[test]
    fn every_core_operand_requires_its_exact_type_and_genuine_global_owner() {
        let original = core();
        for (_, role, kind) in ROLES {
            for change in ["type", "orphan", "foreign", "local", "shadow", "duplicate"] {
                let mut candidate = original.clone();
                let target = candidate
                    .graph
                    .edges
                    .iter()
                    .find(|e| e.kind == role)
                    .unwrap()
                    .target
                    .clone();
                match change {
                    "type" => {
                        candidate
                            .graph
                            .nodes
                            .iter_mut()
                            .find(|n| n.id == target)
                            .unwrap()
                            .type_name = Some(if kind == "Int" { "Text" } else { "Int" }.into())
                    }
                    "orphan" => candidate
                        .graph
                        .edges
                        .retain(|e| e.kind != "has_field" || e.target != target),
                    "foreign" => {
                        let foreign =
                            candidate
                                .graph
                                .add_node("Thing", "Other", None, BTreeMap::new());
                        candidate
                            .graph
                            .edges
                            .iter_mut()
                            .find(|e| e.kind == "has_field" && e.target == target)
                            .unwrap()
                            .source = foreign.id;
                    }
                    "local" => {
                        candidate
                            .graph
                            .nodes
                            .iter_mut()
                            .find(|n| n.id == target)
                            .unwrap()
                            .attributes
                            .insert("function_owner".into(), "Other".into());
                    }
                    "shadow" => {
                        let mut shadow = candidate
                            .graph
                            .nodes
                            .iter()
                            .find(|n| n.id == target)
                            .unwrap()
                            .clone();
                        shadow.id = "shadow".into();
                        candidate.graph.nodes.push(shadow.clone());
                        candidate
                            .graph
                            .edges
                            .iter_mut()
                            .find(|e| e.kind == role)
                            .unwrap()
                            .target = shadow.id;
                    }
                    "duplicate" => candidate.graph.edges.push(
                        candidate
                            .graph
                            .edges
                            .iter()
                            .find(|e| e.kind == "has_field" && e.target == target)
                            .unwrap()
                            .clone(),
                    ),
                    _ => unreachable!(),
                }
                assert!(!check(&candidate).is_empty(), "{role} {change}");
                if let Ok(saved) = parse_ail_core_text(&render_ail_core(&candidate)) {
                    assert!(!check(&saved).is_empty(), "saved {role} {change}");
                }
                rejected_direct_and_saved(&candidate);
            }
        }
    }
    #[test]
    fn process_core_rejects_ambiguous_node_identities() {
        let original = core();
        let process = original
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "ProcessOperation")
            .unwrap();
        let mut identities = vec![process.id.clone()];
        for edge in &original.graph.edges {
            if edge.source == process.id {
                identities.push(edge.target.clone());
            }
            if edge.kind == "process_step" && edge.target == process.id {
                identities.push(edge.source.clone());
            }
            if edge.kind == "has_field" {
                identities.push(edge.source.clone());
            }
        }
        identities.sort();
        identities.dedup();
        let mut accepted = Vec::new();
        for id in identities {
            for first in [false, true] {
                let mut candidate = original.clone();
                let mut duplicate = candidate
                    .graph
                    .nodes
                    .iter()
                    .find(|n| n.id == id)
                    .unwrap()
                    .clone();
                duplicate.name = format!("hidden.{}", duplicate.name);
                if duplicate.kind == "ProcessOperation" {
                    duplicate
                        .attributes
                        .insert("resource".into(), "hidden".into());
                }
                if first {
                    candidate.graph.nodes.insert(0, duplicate);
                } else {
                    candidate.graph.nodes.push(duplicate);
                }
                if check(&candidate).is_empty() {
                    accepted.push(format!("{id} first={first}"));
                } else {
                    assert!(
                        compile_ail_core_bytecode(&candidate).is_err(),
                        "{id} first={first}"
                    );
                }
            }
        }
        assert!(
            accepted.is_empty(),
            "ambiguous process identities accepted: {accepted:?}"
        );
    }
    #[test]
    fn process_core_profiles_owners_and_semantic_secrets_are_checked() {
        let original = core();
        for profile in ["Compiler", "System", "UI", "AgentTool"] {
            let mut candidate = original.clone();
            candidate.package.profile = profile.into();
            rejected_direct_and_saved(&candidate);
        }
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Action")
            .unwrap()
            .attributes
            .insert("kind".into(), "CompilerPass".into());
        rejected_direct_and_saved(&candidate);
        let mut secret = original;
        secret
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        for field in secret.graph.nodes.iter_mut().filter(|n| n.kind == "Field") {
            field.attributes.insert("secret".into(), "true".into());
        }
        assert_eq!(accesses(&secret, "reads").len(), 3);
        assert_eq!(accesses(&secret, "writes").len(), 3);
        assert_eq!(
            check_secret_read_protection(&secret)
                .iter()
                .filter(|d| d.code == "AIL005")
                .count(),
            3
        );
        assert_eq!(
            check_secret_write_protection(&secret)
                .iter()
                .filter(|d| d.code == "AIL002")
                .count(),
            3
        );
    }
    #[test]
    fn argument_bound_counts_argv_zero_and_terminators_before_nul() {
        let path = Path::new("/bin/true");
        assert_eq!(
            argument_bytes(path, &vec![String::new(); 256]).unwrap(),
            path.as_os_str().len() + 1 + 256
        );
        let exact = vec!["x".repeat(MAX_ARGUMENT_BYTES - path.as_os_str().len() - 2)];
        assert_eq!(argument_bytes(path, &exact).unwrap(), MAX_ARGUMENT_BYTES);
        let too_large = vec![format!("{}\0", exact[0])];
        assert!(
            argument_bytes(path, &too_large)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-PROCESS-LIMIT:")
        );
    }
    #[test]
    fn argument_input_and_timeout_failure_priority_is_complete() {
        let mut values = state();
        let mut allocated = 11;
        values.insert(
            "data.arguments".into(),
            format!("[\"\\u0000\",{}]", vec!["\"\""; 256].join(",")),
        );
        values.insert("data.input".into(), "x".repeat(MAX_TEXT_BYTES + 1));
        values.insert("data.timeout".into(), "no".into());
        for (arguments, expected) in [
            (None, "AIL-RUNTIME-PROCESS-LIMIT:"),
            (Some("[\"\\u0000\"]"), "AIL-RUNTIME-PROCESS:"),
            (Some("[]"), "AIL-RUNTIME-TEXT-LIMIT:"),
        ] {
            if let Some(arguments) = arguments {
                values.insert("data.arguments".into(), arguments.into());
            }
            let before = values.clone();
            let error = execute(&command(), &mut values, &mut allocated, &grants()).unwrap_err();
            assert!(error.starts_with(expected), "{error}");
            assert_eq!(values, before);
            assert_eq!(allocated, 11);
        }
        values.insert("data.input".into(), "".into());
        assert!(
            execute(&command(), &mut values, &mut allocated, &grants())
                .unwrap_err()
                .starts_with("AIL-RUNTIME-INT:")
        );
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn empty_results_charge_twice_and_nonzero_exit_is_a_result() {
        let resources = AilProcessResources {
            executables: BTreeMap::from([("tool".into(), PathBuf::from("/bin/sh"))]),
        };
        let mut values = state();
        values.insert("data.arguments".into(), "[\"-c\",\"exit 127\"]".into());
        let original = values.clone();
        let mut allocated = 67_108_864 - 1;
        assert!(
            execute(&command(), &mut values, &mut allocated, &resources)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT-LIMIT:")
        );
        assert_eq!(values, original);
        assert_eq!(allocated, 67_108_864 - 1);
        allocated -= 1;
        execute(&command(), &mut values, &mut allocated, &resources).unwrap();
        assert_eq!(values["data.output"], "");
        assert_eq!(values["data.errors"], "");
        assert_eq!(values["data.status"], "127");
        assert_eq!(allocated, 67_108_864);
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn real_process_preserves_unicode_nul_and_input_timeout_aliases() {
        let mut values = state();
        let mut allocated = 9;
        let original = values["data.input"].clone();
        values.insert("data.alias".into(), original.clone());
        let mut aliases = command();
        aliases
            .operands
            .insert("stdout".into(), "data.input".into());
        aliases
            .operands
            .insert("status".into(), "data.timeout".into());
        execute(&aliases, &mut values, &mut allocated, &grants()).unwrap();
        assert_eq!(values["data.input"], original);
        assert_eq!(values["data.alias"], original);
        assert_eq!(values["data.errors"], "");
        assert_eq!(values["data.timeout"], "0");
        assert_eq!(allocated, 9 + original.len() + 1);
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn second_output_and_combined_arena_failures_preserve_all_results() {
        let resources = AilProcessResources {
            executables: BTreeMap::from([("tool".into(), PathBuf::from("/bin/sh"))]),
        };
        for (script, budget, expected) in [
            ("printf out; printf '\\377' >&2", 0, "AIL-RUNTIME-TEXT:"),
            (
                "printf out; printf err >&2",
                67_108_864 - 5,
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
        ] {
            let mut values = state();
            values.insert(
                "data.arguments".into(),
                format!("[\"-c\",{}]", json_string(script)),
            );
            let before = values.clone();
            let mut allocated = budget;
            let error = execute(&command(), &mut values, &mut allocated, &resources).unwrap_err();
            assert!(error.starts_with(expected), "{error}");
            assert_eq!(values, before);
            assert_eq!(allocated, budget);
        }
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn launch_and_timeout_errors_are_catchable_without_publishing_results() {
        for (path, args, timeout, category) in [
            ("/no/such/ail-process-executable", "[]", "1000", "PROCESS"),
            ("/bin/sh", "[\"-c\",\"/bin/sleep 1\"]", "5", "TIMEOUT"),
        ] {
            let resources = AilProcessResources {
                executables: BTreeMap::from([("tool".into(), PathBuf::from(path))]),
            };
            let mut values = state();
            values.insert("data.arguments".into(), args.into());
            values.insert("data.timeout".into(), timeout.into());
            let before = values.clone();
            let mut allocated = 7;
            let error = execute(&command(), &mut values, &mut allocated, &resources).unwrap_err();
            assert!(
                error.starts_with(&format!("AIL-RUNTIME-{category}:")),
                "{error}"
            );
            assert_eq!(values, before);
            assert_eq!(allocated, 7);
        }
    }
}
