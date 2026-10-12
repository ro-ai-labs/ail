//! Explicit caller-owned standard streams for executable actions.
use super::*;
use crate::expression::IntExpr;
use std::io::{Read, Write};

const MAX_TEXT_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilStandardStreamOperation {
    Read { key: String },
    WriteOutput { source: String },
    WriteError { source: String },
    Invalid(String),
}

/// Optional borrowed endpoints independently grant standard-stream access.
#[derive(Default)]
pub struct AilStandardStreams<'a> {
    pub stdin: Option<&'a mut dyn Read>,
    pub stdout: Option<&'a mut dyn Write>,
    pub stderr: Option<&'a mut dyn Write>,
}

impl AilStandardStreamOperation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Read { key } => format!("the system reads text from standard input into {key}"),
            Self::WriteOutput { source } => {
                format!("the system writes text {source} to standard output")
            }
            Self::WriteError { source } => {
                format!("the system writes text {source} to standard error")
            }
            Self::Invalid(message) => format!("invalid standard-stream operation: {message}"),
        }
    }
}

fn error(message: impl Display) -> String {
    format!("AIL-STDIO-001: {message}")
}
fn field(key: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(key), Ok(IntExpr::Field(parsed)) if parsed == key) {
        Ok(())
    } else {
        Err(error("standard-stream operand must be a field reference"))
    }
}
fn standard_marker(text: &str) -> bool {
    text.strip_prefix("standard").is_some_and(|rest| {
        rest.chars()
            .next()
            .is_none_or(|ch| !ch.is_alphanumeric() && ch != '_')
    })
}
fn recognized_system_bullet(text: &str) -> bool {
    text.strip_prefix("the system reads text from ")
        .is_some_and(standard_marker)
        || text
            .strip_prefix("the system writes text ")
            .is_some_and(|rest| {
                rest.strip_prefix("to ").is_some_and(standard_marker)
                    || rest
                        .split_once(" to ")
                        .is_some_and(|(_, target)| standard_marker(target))
            })
}
/// Recognize standard-stream syntax even in bodies that cannot execute it.
pub(super) fn recognized_bullet(text: &str) -> bool {
    let text = trim_sentence(text);
    if let Some(rest) = text.strip_prefix("the function ") {
        recognized_system_bullet(&format!("the system {rest}"))
    } else {
        recognized_system_bullet(&text)
    }
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilStandardStreamOperation, String>> {
    let text = trim_sentence(text);
    if !recognized_system_bullet(&text) {
        return None;
    }
    Some((|| {
        if let Some(key) = text.strip_prefix("the system reads text from standard input into ") {
            field(key)?;
            return Ok(AilStandardStreamOperation::Read { key: key.into() });
        }
        let (source, target) = text
            .strip_prefix("the system writes text ")
            .and_then(|rest| rest.rsplit_once(" to standard "))
            .ok_or_else(|| error("malformed standard-stream operation"))?;
        field(source)?;
        match target {
            "output" => Ok(AilStandardStreamOperation::WriteOutput {
                source: source.into(),
            }),
            "error" => Ok(AilStandardStreamOperation::WriteError {
                source: source.into(),
            }),
            _ => Err(error(
                "standard-stream output must be standard output or standard error",
            )),
        }
    })())
}
fn components(
    op: &AilStandardStreamOperation,
) -> Result<(&'static str, &'static str, &str), String> {
    match op {
        AilStandardStreamOperation::Read { key } => Ok(("stdin_read", "stream_destination", key)),
        AilStandardStreamOperation::WriteOutput { source } => {
            Ok(("stdout_write", "stream_source", source))
        }
        AilStandardStreamOperation::WriteError { source } => {
            Ok(("stderr_write", "stream_source", source))
        }
        AilStandardStreamOperation::Invalid(message) => Err(error(message)),
    }
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, op) in action.standard_stream_operations.iter().enumerate() {
            let parts = components(op);
            let kind = parts.as_ref().map(|parts| parts.0).unwrap_or("invalid");
            let node = graph.add_node(
                "StandardStreamOperation",
                format!("{}.stream{index}", owner.name),
                None,
                attr(&[("stream_kind", kind)]),
            );
            attach_provenance(
                graph,
                &node,
                format!("action:{}.stream:{index}", owner.name),
            );
            if let Ok((_, edge_kind, key)) = parts {
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
                            format!("action:{}.stream:{index}.operand", owner.name),
                        );
                        target
                    });
                graph.add_edge(edge_kind, &node, &target, BTreeMap::new());
            }
            action_order::append(graph, "stream_step", &owner, &node, BTreeMap::new(), index);
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
        .is_some_and(|v| v.parse::<usize>().is_ok())
        && edge.attributes.iter().all(|(k, v)| {
            matches!(k.as_str(), "step_index" | "operation_index") && v.parse::<usize>().is_ok()
        })
}
fn decode_node(core: &AilCore, node: &Node) -> Result<AilStandardStreamOperation, String> {
    if node.kind != "StandardStreamOperation"
        || node.type_name.is_some()
        || node
            .attributes
            .keys()
            .map(String::as_str)
            .ne(["stream_kind"])
    {
        return Err(error(
            "stream node requires exactly stream_kind and no type annotation",
        ));
    }
    let edge_kind = match node.attributes["stream_kind"].as_str() {
        "stdin_read" => "stream_destination",
        "stdout_write" | "stderr_write" => "stream_source",
        _ => return Err(error("unknown standard-stream operation kind")),
    };
    let owners: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect();
    if owners.len() != 1
        || owners[0].kind != "stream_step"
        || !valid_step(owners[0])
        || !core
            .graph
            .nodes
            .iter()
            .any(|n| n.id == owners[0].source && executable_action(n))
    {
        return Err(error(
            "stream operation requires exactly one ordered executable-action owner",
        ));
    }
    let mut operand = None;
    for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
        if !edge.attributes.is_empty() {
            return Err(error(
                "stream operand and provenance edges require no attributes",
            ));
        }
        let target = core
            .graph
            .nodes
            .iter()
            .find(|n| n.id == edge.target)
            .ok_or_else(|| error("missing standard-stream edge target"))?;
        if edge.kind == "has_provenance" && target.kind == "Provenance" {
            continue;
        }
        if edge.kind != edge_kind || operand.is_some() {
            return Err(error(
                "stream operation requires exactly its one typed field operand",
            ));
        }
        operand = Some(float_operations::scoped_field(core, target, None, "Text").map_err(error)?);
    }
    let key = operand.ok_or_else(|| error("stream operation is missing its Text field operand"))?;
    match node.attributes["stream_kind"].as_str() {
        "stdin_read" => Ok(AilStandardStreamOperation::Read { key }),
        "stdout_write" => Ok(AilStandardStreamOperation::WriteOutput { source: key }),
        "stderr_write" => Ok(AilStandardStreamOperation::WriteError { source: key }),
        _ => unreachable!(),
    }
}
pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilStandardStreamOperation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "stream_step")
        .map(|edge| {
            core.graph
                .nodes
                .iter()
                .find(|n| n.id == edge.target)
                .ok_or_else(|| error("missing stream operation node"))
                .and_then(|node| decode_node(core, node))
                .unwrap_or_else(AilStandardStreamOperation::Invalid)
        })
        .collect()
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for node in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "StandardStreamOperation")
    {
        let result = if matches!(core.package.profile.as_str(), "Application" | "C interop") {
            decode_node(core, node).map(|_| ())
        } else {
            Err(error(
                "standard-stream operations require Application or C interop profile",
            ))
        };
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-STDIO-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = core.graph.nodes.iter().find(|n| n.id == edge.source);
        let target = core.graph.nodes.iter().find(|n| n.id == edge.target);
        let valid = match edge.kind.as_str() {
            "stream_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "StandardStreamOperation")
                    && valid_step(edge)
            }
            "stream_destination" | "stream_source" => {
                source.is_some_and(|n| n.kind == "StandardStreamOperation")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-STDIO-001",
                    format!("invalid {} standard-stream edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace(
    op: &AilStandardStreamOperation,
    alias: &str,
) -> AilStandardStreamOperation {
    let qualify = |key: &str| format!("{}.{key}", runtime_subject_key(alias));
    match op {
        AilStandardStreamOperation::Read { key } => {
            AilStandardStreamOperation::Read { key: qualify(key) }
        }
        AilStandardStreamOperation::WriteOutput { source } => {
            AilStandardStreamOperation::WriteOutput {
                source: qualify(source),
            }
        }
        AilStandardStreamOperation::WriteError { source } => {
            AilStandardStreamOperation::WriteError {
                source: qualify(source),
            }
        }
        AilStandardStreamOperation::Invalid(message) => {
            AilStandardStreamOperation::Invalid(message.clone())
        }
    }
}
pub(super) fn instruction(
    op: &AilStandardStreamOperation,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let (kind, _, key) = components(op)?;
    let (opcode, operand) = match kind {
        "stdin_read" => ("STDIN_READ_TEXT", "key"),
        "stdout_write" => ("STDOUT_WRITE_TEXT", "source"),
        _ => ("STDERR_WRITE_TEXT", "source"),
    };
    let instruction = AilBytecodeInstruction::new(opcode, &[(operand, key.into())]);
    verify(&instruction)?;
    if runtime_field_type_name(document, key) != Some("Text") {
        return Err(error(
            "standard-stream field must be a declared global Text field",
        ));
    }
    Ok(instruction)
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let operand = match instruction.opcode.as_str() {
        "STDIN_READ_TEXT" => "key",
        "STDOUT_WRITE_TEXT" | "STDERR_WRITE_TEXT" => "source",
        _ => return Err(error("unknown standard-stream instruction")),
    };
    if instruction
        .operands
        .keys()
        .map(String::as_str)
        .ne([operand])
    {
        return Err(error(
            "standard-stream instruction requires exactly one field operand",
        ));
    }
    field(&instruction.operands[operand])
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let edge_kind = match kind {
        "reads" => "stream_source",
        "writes" => "stream_destination",
        _ => return Vec::new(),
    };
    let mut accesses = Vec::new();
    for step in core.graph.edges.iter().filter(|e| e.kind == "stream_step") {
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|e| e.source == step.target && e.kind == edge_kind)
        {
            accesses.push((edge, step.source.as_str()));
        }
    }
    accesses
}
fn retry_io<T>(mut operation: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    loop {
        match operation() {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}
fn io_error(operation: &str) -> String {
    format!("AIL-RUNTIME-IO: standard-stream {operation} failed")
}
pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
    allocated: &mut usize,
    streams: &mut AilStandardStreams<'_>,
) -> Result<(), String> {
    verify(instruction)?;
    if instruction.opcode == "STDIN_READ_TEXT" {
        let reader = streams
            .stdin
            .as_deref_mut()
            .ok_or("AIL-RUNTIME-CAPABILITY: standard input is not granted")?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(MAX_TEXT_BYTES + 1)
            .map_err(|_| "AIL-RUNTIME-TEXT-LIMIT: unable to allocate standard input storage")?;
        let mut buffer = [0; 8192];
        while bytes.len() < MAX_TEXT_BYTES + 1 {
            let remaining = (MAX_TEXT_BYTES + 1 - bytes.len()).min(buffer.len());
            let count =
                retry_io(|| reader.read(&mut buffer[..remaining])).map_err(|_| io_error("read"))?;
            if count == 0 {
                break;
            }
            if count > remaining {
                return Err(io_error("read"));
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
        if bytes.len() > MAX_TEXT_BYTES {
            return Err("AIL-RUNTIME-TEXT-LIMIT: standard input exceeds 1048576 bytes".into());
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| "AIL-RUNTIME-TEXT: standard input is not valid UTF-8")?;
        text_operations::reserve_result(text.len(), allocated)?;
        state.insert(instruction.operands["key"].clone(), text);
    } else {
        let writer = if instruction.opcode == "STDOUT_WRITE_TEXT" {
            streams
                .stdout
                .as_deref_mut()
                .ok_or("AIL-RUNTIME-CAPABILITY: standard output is not granted")?
        } else {
            streams
                .stderr
                .as_deref_mut()
                .ok_or("AIL-RUNTIME-CAPABILITY: standard error is not granted")?
        };
        let text = text_operations::read_text(state, &instruction.operands["source"])?;
        if text.is_empty() {
            return Ok(());
        }
        let mut remaining = text.as_bytes();
        while !remaining.is_empty() {
            let count = retry_io(|| writer.write(remaining)).map_err(|_| io_error("write"))?;
            if count == 0 || count > remaining.len() {
                return Err(io_error("write"));
            }
            remaining = &remaining[count..];
        }
        retry_io(|| writer.flush()).map_err(|_| io_error("flush"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Error, ErrorKind};

    fn read() -> AilBytecodeInstruction {
        AilBytecodeInstruction::new("STDIN_READ_TEXT", &[("key", "data.text".into())])
    }
    fn write(opcode: &str) -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(opcode, &[("source", "data.text".into())])
    }
    fn state(text: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("data.text".into(), text.into())])
    }

    struct ShortReader {
        bytes: Cursor<Vec<u8>>,
        interrupted: bool,
    }
    impl Read for ShortReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(Error::new(ErrorKind::Interrupted, "private reader detail"));
            }
            let length = buffer.len().min(1);
            self.bytes.read(&mut buffer[..length])
        }
    }
    #[derive(Default)]
    struct Writer {
        bytes: Vec<u8>,
        calls: usize,
        flushes: usize,
        failure: Option<ErrorKind>,
        flush_failure: bool,
        flush_interrupted: bool,
        fail_after: Option<usize>,
        zero: bool,
    }
    impl Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.calls += 1;
            if self
                .fail_after
                .is_some_and(|limit| self.bytes.len() >= limit)
            {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "private partial-write detail",
                ));
            }
            if let Some(kind) = self.failure.take() {
                return Err(Error::new(kind, "private writer detail"));
            }
            if self.zero {
                return Ok(0);
            }
            let length = bytes.len().min(2);
            self.bytes.extend_from_slice(&bytes[..length]);
            Ok(length)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.flushes += 1;
            if self.flush_interrupted {
                self.flush_interrupted = false;
                return Err(Error::new(ErrorKind::Interrupted, "private flush detail"));
            }
            if self.flush_failure {
                Err(Error::new(ErrorKind::BrokenPipe, "private flush detail"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn parser_roundtrips_streams_and_does_not_claim_file_writes() {
        for source in [
            "the system reads text from standard input into data.text",
            "the system writes text data.text to standard output",
            "the system writes text data.text to standard error",
        ] {
            let op = parse_bullet(&format!("{source}."))
                .expect("recognized operation")
                .unwrap();
            assert_eq!(op.source(), source);
            assert!(recognized_bullet(
                &source.replace("the system", "the function")
            ));
        }
        assert!(parse_bullet("the system writes text data.text to file resource output").is_none());
    }

    #[test]
    fn read_retries_short_interrupted_utf8_and_preserves_cursor_and_alias() {
        let original = "a\0🙂\r\n";
        let mut reader = ShortReader {
            bytes: Cursor::new(original.as_bytes().to_vec()),
            interrupted: false,
        };
        let mut streams = AilStandardStreams {
            stdin: Some(&mut reader),
            ..Default::default()
        };
        let mut values = state("old");
        values.insert("data.alias".into(), "old".into());
        let mut allocated = 7;
        execute(&read(), &mut values, &mut allocated, &mut streams).unwrap();
        assert_eq!(values["data.text"], original);
        assert_eq!(values["data.alias"], "old");
        assert_eq!(allocated, 7 + original.len());
        execute(&read(), &mut values, &mut allocated, &mut streams).unwrap();
        assert_eq!(values["data.text"], "");
        assert_eq!(allocated, 8 + original.len());
    }

    #[test]
    fn writes_retry_short_interrupted_output_then_flush_exact_bytes() {
        let mut output = Writer {
            failure: Some(ErrorKind::Interrupted),
            ..Default::default()
        };
        let mut errors = Writer::default();
        let mut values = state("é\0🙂\r\n");
        let mut allocated = 13;
        {
            let mut streams = AilStandardStreams {
                stdin: None,
                stdout: Some(&mut output),
                stderr: Some(&mut errors),
            };
            execute(
                &write("STDOUT_WRITE_TEXT"),
                &mut values,
                &mut allocated,
                &mut streams,
            )
            .unwrap();
            execute(
                &write("STDERR_WRITE_TEXT"),
                &mut values,
                &mut allocated,
                &mut streams,
            )
            .unwrap();
        }
        assert_eq!(output.bytes, values["data.text"].as_bytes());
        assert_eq!(errors.bytes, output.bytes);
        assert_eq!((output.flushes, errors.flushes, allocated), (1, 1, 13));
    }

    #[test]
    fn grants_are_endpoint_specific_and_checked_before_missing_operands() {
        let mut input = Cursor::new(b"unused".to_vec());
        let mut output = Writer::default();
        let mut errors = Writer::default();
        let mut allocated = 0;
        for operation in [
            read(),
            write("STDOUT_WRITE_TEXT"),
            write("STDERR_WRITE_TEXT"),
        ] {
            let mut streams = AilStandardStreams {
                stdin: (operation.opcode != "STDIN_READ_TEXT")
                    .then_some(&mut input as &mut dyn Read),
                stdout: (operation.opcode != "STDOUT_WRITE_TEXT")
                    .then_some(&mut output as &mut dyn Write),
                stderr: (operation.opcode != "STDERR_WRITE_TEXT")
                    .then_some(&mut errors as &mut dyn Write),
            };
            let message = execute(
                &operation,
                &mut BTreeMap::new(),
                &mut allocated,
                &mut streams,
            )
            .unwrap_err();
            assert!(message.starts_with("AIL-RUNTIME-CAPABILITY:"), "{message}");
        }
        assert_eq!(
            (input.position(), output.calls, errors.calls, allocated),
            (0, 0, 0, 0)
        );
    }

    #[test]
    fn read_size_utf8_and_arena_checks_commit_atomically_after_consumption() {
        for (bytes, initial, family, consumed) in [
            (
                vec![0xff; MAX_TEXT_BYTES + 10],
                9,
                "TEXT-LIMIT",
                MAX_TEXT_BYTES + 1,
            ),
            (vec![0xff, b'x'], 9, "TEXT", 2),
            (b"okay".to_vec(), 67_108_862, "TEXT-LIMIT", 4),
            (Vec::new(), 67_108_864, "TEXT-LIMIT", 0),
        ] {
            let mut reader = Cursor::new(bytes);
            let mut values = state("preserved");
            let mut allocated = initial;
            let message = execute(
                &read(),
                &mut values,
                &mut allocated,
                &mut AilStandardStreams {
                    stdin: Some(&mut reader),
                    ..Default::default()
                },
            )
            .unwrap_err();
            assert!(
                message.starts_with(&format!("AIL-RUNTIME-{family}:")),
                "{message}"
            );
            assert_eq!(values["data.text"], "preserved");
            assert_eq!(allocated, initial);
            assert_eq!(reader.position(), consumed as u64);
        }
        let mut reader = Cursor::new(vec![b'x'; MAX_TEXT_BYTES]);
        let mut values = state("old");
        let mut allocated = 67_108_864 - MAX_TEXT_BYTES;
        execute(
            &read(),
            &mut values,
            &mut allocated,
            &mut AilStandardStreams {
                stdin: Some(&mut reader),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(allocated, 67_108_864);
        assert_eq!(values["data.text"].len(), MAX_TEXT_BYTES);
    }

    #[test]
    fn write_validation_and_empty_values_perform_no_host_io() {
        let mut writer = Writer::default();
        let mut allocated = 67_108_864;
        for (mut values, family) in [
            (BTreeMap::new(), Some("TEXT")),
            (state(&"x".repeat(MAX_TEXT_BYTES + 1)), Some("TEXT-LIMIT")),
            (state(""), None),
        ] {
            let result = execute(
                &write("STDOUT_WRITE_TEXT"),
                &mut values,
                &mut allocated,
                &mut AilStandardStreams {
                    stdout: Some(&mut writer),
                    ..Default::default()
                },
            );
            if let Some(family) = family {
                assert!(
                    result
                        .unwrap_err()
                        .starts_with(&format!("AIL-RUNTIME-{family}:"))
                );
            } else {
                result.unwrap();
            }
        }
        assert_eq!(
            (writer.calls, writer.flushes, allocated),
            (0, 0, 67_108_864)
        );
    }

    #[test]
    fn write_zero_io_and_flush_failures_are_sanitized_and_keep_partial_effects() {
        for mut writer in [
            Writer {
                zero: true,
                ..Default::default()
            },
            Writer {
                failure: Some(ErrorKind::WouldBlock),
                ..Default::default()
            },
            Writer {
                flush_failure: true,
                ..Default::default()
            },
        ] {
            let mut values = state("four");
            let mut allocated = 19;
            let message = execute(
                &write("STDERR_WRITE_TEXT"),
                &mut values,
                &mut allocated,
                &mut AilStandardStreams {
                    stderr: Some(&mut writer),
                    ..Default::default()
                },
            )
            .unwrap_err();
            assert!(message.starts_with("AIL-RUNTIME-IO:"), "{message}");
            assert!(!message.contains("private"));
            assert_eq!(values["data.text"], "four");
            assert_eq!(allocated, 19);
            assert_eq!(
                writer.bytes,
                if writer.flush_failure {
                    b"four".to_vec()
                } else {
                    Vec::new()
                }
            );
        }
    }

    #[test]
    fn protocol_rejects_wrong_arity_and_expression_operands() {
        for operation in [
            read(),
            write("STDOUT_WRITE_TEXT"),
            write("STDERR_WRITE_TEXT"),
        ] {
            verify(&operation).unwrap();
            let mut extra = operation.clone();
            extra.operands.insert("resource".into(), "ambient".into());
            assert!(verify(&extra).is_err());
            let mut bad = operation;
            *bad.operands.values_mut().next().unwrap() = "data.text + 1".into();
            assert!(verify(&bad).is_err());
        }
    }

    #[test]
    fn read_errors_precede_decoding_and_exact_size_requires_confirming_eof() {
        struct ErrorAfterInput(Cursor<Vec<u8>>);
        impl Read for ErrorAfterInput {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if self.0.position() as usize == self.0.get_ref().len() {
                    Err(Error::new(ErrorKind::WouldBlock, "private read detail"))
                } else {
                    self.0.read(buffer)
                }
            }
        }
        for bytes in [vec![0xff], vec![b'x'; MAX_TEXT_BYTES]] {
            let expected_consumed = bytes.len();
            let mut reader = ErrorAfterInput(Cursor::new(bytes));
            let mut values = state("preserved");
            let mut allocated = 5;
            let message = execute(
                &read(),
                &mut values,
                &mut allocated,
                &mut AilStandardStreams {
                    stdin: Some(&mut reader),
                    ..Default::default()
                },
            )
            .unwrap_err();
            assert!(message.starts_with("AIL-RUNTIME-IO:"), "{message}");
            assert!(!message.contains("private"));
            assert_eq!(reader.0.position(), expected_consumed as u64);
            assert_eq!(values["data.text"], "preserved");
            assert_eq!(allocated, 5);
        }
    }

    #[test]
    fn partial_writes_persist_and_interrupted_flush_retries() {
        let mut partial = Writer {
            fail_after: Some(2),
            ..Default::default()
        };
        let mut values = state("abcdef");
        let mut allocated = 7;
        let message = execute(
            &write("STDOUT_WRITE_TEXT"),
            &mut values,
            &mut allocated,
            &mut AilStandardStreams {
                stdout: Some(&mut partial),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(message.starts_with("AIL-RUNTIME-IO:"));
        assert_eq!(partial.bytes, b"ab");
        assert_eq!(partial.flushes, 0);
        let mut writer = Writer {
            flush_interrupted: true,
            ..Default::default()
        };
        execute(
            &write("STDOUT_WRITE_TEXT"),
            &mut values,
            &mut allocated,
            &mut AilStandardStreams {
                stdout: Some(&mut writer),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(writer.bytes, b"abcdef");
        assert_eq!((writer.flushes, allocated), (2, 7));
    }

    #[test]
    fn malformed_stream_forms_are_recognized_without_claiming_unrelated_prose() {
        for source in [
            "the system reads text from standard input",
            "the system reads text from standard input into bare",
            "the system reads text from standard error into data.text",
            "the system writes text to standard output",
            "the system writes text data.text to standard",
            "the system writes text data.text to standard:",
            "the system reads text from standard:",
            "the system writes text data.text to standard input",
            "the system writes text data.text to standard output extra",
            "the system writes text data.text + 1 to standard error",
        ] {
            for suffix in ["", "."] {
                let source = format!("{source}{suffix}");
                assert!(parse_bullet(&source).unwrap().is_err(), "{source}");
                assert!(
                    recognized_bullet(&source.replace("the system", "the function")),
                    "{source}"
                );
            }
        }
        for source in [
            "the system reads text from file resource input into data.text",
            "the system writes text data.text to file resource output",
            "the system reads standards",
            "the system reads text from standardized storage",
            "the system joins text data.text and data.other into data.text",
        ] {
            assert!(parse_bullet(source).is_none(), "{source}");
            assert!(!recognized_bullet(source), "{source}");
        }
    }

    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Streams manages text.\n\nA Data has:\n- text: Text\n\nAction: Run.\nWhen Run happens:\n- the system reads text from standard input into data.text\n- the system writes text data.text to standard output\n- the system writes text data.text to standard error\n- the system records a trace event named Finished\n";
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        )
    }

    #[test]
    fn source_saved_and_rendered_core_keep_order_and_only_fields_namespace() {
        let original = core_fixture();
        assert!(
            check_ail_core(&original).is_empty(),
            "{:?}",
            check_ail_core(&original)
        );
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let rendered = elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, &render_ail_spec_from_core(&original)).unwrap(),
        );
        for candidate in [original, saved, rendered] {
            assert!(
                check_ail_core(&candidate).is_empty(),
                "{:?}",
                check_ail_core(&candidate)
            );
            let program = compile_ail_core_bytecode(&candidate).unwrap();
            assert_eq!(
                program.actions["Run"]
                    .instructions
                    .iter()
                    .filter(|i| i.opcode.starts_with("STD"))
                    .map(|i| i.opcode.as_str())
                    .collect::<Vec<_>>(),
                ["STDIN_READ_TEXT", "STDOUT_WRITE_TEXT", "STDERR_WRITE_TEXT"]
            );
            let owner = candidate.graph.find_node("Action", "Run").unwrap();
            let operations = from_core(&candidate, owner);
            assert_eq!(operations.len(), 3);
            for operation in operations {
                let qualified = namespace(&operation, "Child");
                assert_eq!(
                    qualified.source(),
                    operation.source().replace("data.text", "child.data.text")
                );
            }
        }
    }

    #[test]
    fn core_rejects_duplicate_edges_and_ignored_metadata_direct_and_saved() {
        let original = core_fixture();
        let rejected = |core: &AilCore| {
            assert!(!check(core).is_empty());
            assert!(!check(&parse_ail_core_text(&render_ail_core(core)).unwrap()).is_empty());
        };
        for kind in ["stream_step", "stream_source", "stream_destination"] {
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
            rejected(&duplicate);
            let mut extra = original.clone();
            extra
                .graph
                .edges
                .iter_mut()
                .find(|e| e.kind == kind)
                .unwrap()
                .attributes
                .insert("hidden".into(), "ignored".into());
            rejected(&extra);
        }
        for (name, value) in [
            ("resource", "ambient"),
            ("stream_kind", "append"),
            ("function_owner", "Fn"),
        ] {
            let mut candidate = original.clone();
            candidate
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == "StandardStreamOperation")
                .unwrap()
                .attributes
                .insert(name.into(), value.into());
            rejected(&candidate);
        }
        let mut typed = original.clone();
        typed
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "StandardStreamOperation")
            .unwrap()
            .type_name = Some("Text".into());
        rejected(&typed);
        let mut hidden = original;
        let owner = hidden.graph.find_node("Action", "Run").unwrap().clone();
        let operation = hidden
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "StandardStreamOperation")
            .unwrap()
            .clone();
        hidden
            .graph
            .add_edge("reads", &owner, &operation, BTreeMap::new());
        rejected(&hidden);
    }

    #[test]
    fn core_requires_genuine_unique_global_storage_and_executable_owners() {
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
        for change in [
            "orphan",
            "foreign",
            "local",
            "type",
            "shadow",
            "duplicate",
            "owner",
        ] {
            let mut candidate = original.clone();
            let field = candidate
                .graph
                .nodes
                .iter()
                .find(|n| n.kind == "Field")
                .unwrap()
                .clone();
            match change {
                "orphan" => candidate.graph.edges.retain(|e| e.kind != "has_field"),
                "foreign" => {
                    let other = candidate
                        .graph
                        .add_node("Thing", "Other", None, BTreeMap::new());
                    candidate
                        .graph
                        .edges
                        .iter_mut()
                        .find(|e| e.kind == "has_field")
                        .unwrap()
                        .source = other.id;
                }
                "local" => {
                    candidate
                        .graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == field.id)
                        .unwrap()
                        .attributes
                        .insert("function_owner".into(), "Hidden".into());
                }
                "type" => {
                    candidate
                        .graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == field.id)
                        .unwrap()
                        .type_name = Some("Int".into());
                }
                "shadow" => {
                    let mut shadow = field;
                    shadow.id = "shadow".into();
                    candidate.graph.nodes.push(shadow.clone());
                    candidate
                        .graph
                        .edges
                        .iter_mut()
                        .find(|e| e.kind == "stream_source")
                        .unwrap()
                        .target = shadow.id;
                }
                "duplicate" => candidate.graph.edges.push(
                    candidate
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.kind == "has_field")
                        .unwrap()
                        .clone(),
                ),
                "owner" => candidate.graph.edges.retain(|e| e.kind != "stream_step"),
                _ => unreachable!(),
            }
            assert!(!check(&candidate).is_empty(), "{change}");
        }
    }

    #[test]
    fn semantic_secret_accesses_survive_removal_of_descriptive_edges() {
        let mut core = core_fixture();
        core.graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        core.graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field")
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        assert_eq!(accesses(&core, "reads").len(), 2);
        assert_eq!(accesses(&core, "writes").len(), 1);
        assert!(
            check_secret_read_protection(&core)
                .iter()
                .any(|d| d.code == "AIL005")
        );
        assert!(
            check_secret_write_protection(&core)
                .iter()
                .any(|d| d.code == "AIL002")
        );
    }
}
