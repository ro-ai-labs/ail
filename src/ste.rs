//! Executable controlled English between checked Core and verified bytecode.
//!
//! This is AIL's programming profile based on ASD-STE100 procedural writing,
//! not a validator for arbitrary STE prose or a claim of full ASD conformity.
//! Templates are short, imperative and unambiguous. Technical data live outside
//! the sentences, so prose, Unicode, and escaped newlines cannot inject code.

use std::collections::BTreeMap;

use crate::ail::{
    AilBytecodeAction, AilBytecodeInstruction, AilBytecodeProgram, AilJsonValue,
    ail_bytecode_required_operands, parse_ail_bytecode, parse_ail_json, render_ail_bytecode,
    verify_ail_bytecode,
};
use crate::core_model::json_string;

pub const STE_HEADER: &str = "AIL-STE 1";

// A placeholder is a technical-data identifier, never arbitrary English.
// Operations which currently only record contracts deliberately say "Record".
const TEMPLATES: &[(&str, &str)] = &[
    (
        "VALUE_CONSTRUCT",
        "Construct {type} variant {variant} with arguments {arguments} into field {key}.",
    ),
    (
        "VALUE_TEST",
        "Test {type} variant {variant} of field {source} into field {key}.",
    ),
    (
        "VALUE_GET",
        "Get field {field} of {type} variant {variant} from field {source} into field {key}.",
    ),
    (
        "BYTES_LENGTH",
        "Count bytes in field {source} into field {key}.",
    ),
    (
        "BYTES_GET",
        "Get byte at index {index} from field {source} into field {key}.",
    ),
    (
        "BYTES_SLICE",
        "Slice bytes in field {source} from {start} to {end} into field {key}.",
    ),
    (
        "BYTES_CONCAT",
        "Join bytes in field {left} and field {right} into field {key}.",
    ),
    (
        "BYTES_SET",
        "Set byte at index {index} in field {source} to value {value} into field {key}.",
    ),
    (
        "BYTES_FROM_UTF8",
        "Encode UTF-8 text from field {source} into bytes field {key}.",
    ),
    (
        "BYTES_TO_UTF8",
        "Decode UTF-8 bytes from field {source} into text field {key}.",
    ),
    (
        "BYTES_FROM_HEX",
        "Decode hexadecimal text from field {source} into bytes field {key}.",
    ),
    (
        "BYTES_TO_HEX",
        "Encode bytes from field {source} into hexadecimal text field {key}.",
    ),
    (
        "BYTES_COMPARE",
        "Compare bytes in field {left} with field {right} into field {key}.",
    ),
    (
        "FILE_READ_BYTES",
        "Read bytes from file resource {resource} into field {key}.",
    ),
    (
        "FILE_WRITE_BYTES",
        "Write bytes from field {source} to file resource {resource}.",
    ),
    ("ACTION_BEGIN", "Record the start of action {action}."),
    ("LABEL", "Mark label {name}."),
    (
        "TRY_BEGIN",
        "Begin recovery for errors {errors} into field {key}.",
    ),
    ("RECOVER", "Run the recovery handler."),
    ("TRY_END", "End recovery."),
    ("RAISE", "Raise error {category}."),
    (
        "BRANCH_FIELD_EQUALS",
        "If field {key} equals {value}, go to label {label}.",
    ),
    ("JUMP", "Go to label {label}."),
    (
        "BRANCH_INT",
        "If integer comparison {left} {comparison} {right} is true, go to label {label}.",
    ),
    ("CALL_ACTION", "Do action {target}."),
    ("REPEAT_ACTION", "Do action {target} {count} times."),
    ("ADD_INT_FIELD", "Add {delta} to field {key}."),
    (
        "EVAL_INT",
        "Evaluate integer expression {expression} into field {key}.",
    ),
    ("OUTPUT_FIELD", "Select output field {key}."),
    (
        "JSON_SET",
        "Set {type} value {value} at JSON path {path} in field {source} into field {key}.",
    ),
    (
        "JSON_REMOVE",
        "Remove value at JSON path {path} from field {source} into field {key}.",
    ),
    (
        "JSON_GET",
        "Get {type} value at JSON path {path} from field {source} into field {key}.",
    ),
    (
        "JSON_COUNT",
        "Count entries in JSON field {source} into field {key}.",
    ),
    (
        "JSON_KIND",
        "Get JSON kind from field {source} into field {key}.",
    ),
    (
        "JSON_KEYS",
        "Get JSON keys from field {source} into field {key}.",
    ),
    (
        "JSON_PARSE",
        "Parse JSON text {source} as type {type} into field {key}.",
    ),
    (
        "JSON_FORMAT",
        "Format {type} field {source} as JSON text into field {key}.",
    ),
    (
        "DIRECTORY_LIST_NAMES",
        "List names in directory resource {resource} at field {path} into field {key}.",
    ),
    (
        "DIRECTORY_READ_TEXT",
        "Read text from directory resource {resource} at field {path} into field {key}.",
    ),
    (
        "DIRECTORY_WRITE_TEXT",
        "Write text from field {source} to directory resource {resource} at field {path}.",
    ),
    (
        "PROCESS_RUN",
        "Run process {resource} with arguments {arguments}, input {input}, limit {timeout_ms} milliseconds, output {stdout}, errors {stderr}, and status {status}.",
    ),
    (
        "STDIN_READ_TEXT",
        "Read text from standard input into field {key}.",
    ),
    (
        "STDOUT_WRITE_TEXT",
        "Write text from field {source} to standard output.",
    ),
    (
        "STDERR_WRITE_TEXT",
        "Write text from field {source} to standard error.",
    ),
    (
        "FILE_READ_TEXT",
        "Read text from file resource {resource} into field {key}.",
    ),
    (
        "FILE_WRITE_TEXT",
        "Write text from field {source} to file resource {resource}.",
    ),
    (
        "ASSIGN_LITERAL",
        "Assign literal {value} of type {type} to field {key}.",
    ),
    (
        "LIST_EMPTY",
        "Create empty list of type {type} into field {key}.",
    ),
    (
        "LIST_LENGTH",
        "Count items in {type} list {source} into field {key}.",
    ),
    (
        "LIST_GET",
        "Get item {index} from {type} list {source} into field {key}.",
    ),
    (
        "LIST_APPEND",
        "Append value {value} to {type} list {source} into field {key}.",
    ),
    (
        "LIST_SET",
        "Set item {index} in {type} list {source} to value {value} into field {key}.",
    ),
    (
        "LIST_CONCAT",
        "Concatenate {type} list {source} and list {other} into field {key}.",
    ),
    (
        "LIST_SLICE",
        "Slice {type} list {source} from integer {start} to integer {end} into field {key}.",
    ),
    (
        "LIST_REVERSE",
        "Reverse {type} list {source} into field {key}.",
    ),
    (
        "LIST_SORT",
        "Sort {type} list {source} in {order} order into field {key}.",
    ),
    (
        "TEXT_JOIN",
        "Join text {left} and {right} into field {key}.",
    ),
    (
        "TEXT_LENGTH",
        "Count characters in text {source} into field {key}.",
    ),
    (
        "TEXT_SLICE",
        "Slice text {source} from integer {start} to integer {end} into field {key}.",
    ),
    (
        "TEXT_SPLIT",
        "Split text in field {source} by field {separator} into field {key}.",
    ),
    (
        "TEXT_JOIN_LIST",
        "Join text list in field {source} with field {separator} into field {key}.",
    ),
    (
        "TEXT_REPLACE",
        "Replace text from field {needle} in field {source} with field {replacement} into field {key}.",
    ),
    (
        "TEXT_TRIM",
        "Trim whitespace from field {source} into field {key}.",
    ),
    (
        "TEXT_FIND",
        "Find text {needle} in text {source} into field {key}.",
    ),
    (
        "REQUIRE_EXISTS",
        "Check that field {key} exists for rule {rule} with failure {failure}.",
    ),
    (
        "REQUIRE_FIELD_NOT_EQUALS",
        "Check that field {key} differs from {value} for rule {rule} with failure {failure}.",
    ),
    (
        "REQUIRE_FIELD_IN",
        "Check that field {key} is in list {values} for rule {rule} with failure {failure}.",
    ),
    (
        "REQUIRE_FIELD_AFTER",
        "Check that field {source} is after field {key} for rule {rule} with failure {failure}.",
    ),
    ("OBSERVE_RULE", "Record rule {rule}."),
    ("READ_FIELD", "Record field read {key}."),
    ("READ_EFFECT", "Record read effect {text}."),
    ("SET_FIELD", "Set field {key} to {value}."),
    ("COPY_FIELD", "Copy field {source} to field {key}."),
    (
        "WRITE_FIELD",
        "Record field write {key} with description {text}.",
    ),
    ("EFFECT", "Record effect {text}."),
    (
        "FUNCTION_BEGIN",
        "Record the start of function {function} with description {label}.",
    ),
    (
        "FUNCTION_INPUT",
        "Record function input {name} with type {type}.",
    ),
    (
        "FUNCTION_OUTPUT",
        "Record function output {name} with type {type}.",
    ),
    ("FUNCTION_BRANCH", "Record function condition {condition}."),
    (
        "FUNCTION_CALL",
        "Record function call {target} with description {text}.",
    ),
    ("FUNCTION_RETURN", "Record function result {value}."),
    (
        "INTEGER_FUNCTION",
        "Start integer function with parameters {parameters}.",
    ),
    (
        "CALL_INT",
        "Call integer function {target} with arguments {arguments} into field {key}.",
    ),
    ("RETURN_INT", "Return integer expression {expression}."),
    (
        "VALUE_FUNCTION",
        "Start value function with parameters {parameters}, locals {locals}, and result type {result}.",
    ),
    (
        "CALL_VALUE",
        "Call value function {target} with arguments {arguments} into field {key}.",
    ),
    (
        "RETURN_VALUE",
        "Return {type} value from expression {expression}.",
    ),
    (
        "EVAL_BOOL",
        "Compute Boolean expression {expression} into field {key}.",
    ),
    (
        "BRANCH_BOOL",
        "If Boolean expression {expression} is true, go to label {label}.",
    ),
    (
        "EVAL_FLOAT",
        "Compute floating expression {expression} into field {key}.",
    ),
    (
        "BRANCH_FLOAT",
        "If floating comparison {left} {comparison} {right} is true, go to label {label}.",
    ),
    (
        "INT_TO_FLOAT",
        "Convert integer field {source} into floating field {key}.",
    ),
    (
        "FLOAT_TO_INT",
        "Convert floating field {source} into integer field {key}.",
    ),
    ("OPTION_MAP", "Map the option with the supplied result."),
    (
        "TOOL_BEGIN",
        "Record the start of tool {tool} with description {label}.",
    ),
    ("TOOL_REQUIREMENT", "Record tool requirement {text}."),
    (
        "TOOL_INPUT",
        "Record tool input {name} with type {type} and secret status {secret}.",
    ),
    (
        "TOOL_OUTPUT",
        "Record tool output {name} with type {type} and secret status {secret}.",
    ),
    ("TOOL_READ", "Record tool read {text}."),
    ("TOOL_CALL", "Record tool call {target}."),
    ("TOOL_WRITE", "Record tool write {text}."),
    ("TOOL_PERMISSION", "Record tool permission {text}."),
    ("TOOL_APPROVAL", "Record tool approval {text}."),
    (
        "TOOL_SECRET_PROTECTION",
        "Record tool secret protection {text}.",
    ),
    (
        "SYSTEM_BEGIN",
        "Record the start of component {component} with description {label}.",
    ),
    (
        "SYSTEM_RESOURCE",
        "Record resource {name} with type {type}.",
    ),
    ("SYSTEM_OWNS", "Record ownership of resource {resource}."),
    (
        "SYSTEM_BORROWS",
        "Record shared access to resource {resource}.",
    ),
    (
        "SYSTEM_MUTABLY_BORROWS",
        "Record exclusive access to resource {resource}.",
    ),
    (
        "SYSTEM_REGION",
        "Record region {region} for resource {resource}.",
    ),
    (
        "SYSTEM_LAYOUT",
        "Record layout {layout} for resource {resource}.",
    ),
    (
        "SYSTEM_ALLOCATION",
        "Record placement {placement} for resource {resource}.",
    ),
    (
        "SYSTEM_LOCK_GUARD",
        "Record lock {lock} for resource {resource}.",
    ),
    ("SYSTEM_CONTEXT", "Record context {name}."),
    (
        "SYSTEM_INTERRUPT_PRIORITY",
        "Record interrupt priority {priority} for context {context}.",
    ),
    (
        "SYSTEM_INTERRUPT_MASK",
        "Record interrupt mask {mask} for context {context}.",
    ),
    (
        "SYSTEM_SCHEDULER_TASK",
        "Record task {task} in context {context}.",
    ),
    (
        "SYSTEM_TASK_PRIORITY",
        "Record priority {priority} for task {task}.",
    ),
    (
        "SYSTEM_TASK_TIMING",
        "Record deadline {deadline} and budget {budget} for task {task}.",
    ),
    ("SYSTEM_CAPABILITY", "Record system capability {text}."),
    ("SYSTEM_EFFECT", "Record system effect {text}."),
    (
        "PASS_BEGIN",
        "Record the start of compiler pass {pass} with description {label} for purpose {purpose}.",
    ),
    (
        "PASS_INPUT",
        "Record compiler input {name} with type {type}.",
    ),
    (
        "PASS_OUTPUT",
        "Record compiler output {name} with type {type}.",
    ),
    ("PASS_READ", "Record compiler read {text}."),
    ("PASS_STEP", "Record compiler step {text}."),
    ("PASS_WRITE", "Record compiler write {text}."),
    (
        "CORE_INFER_READ_PERMISSIONS",
        "Infer {permission_kind} permissions through {edge} from {actor_kinds} to {target_kind} with secret policy {secret_policy}.",
    ),
    ("ASSERT_GUARANTEE", "Record guarantee {text}."),
    ("EMIT_TRACE", "Record trace event {event}."),
    ("RETURN_SUCCESS", "Return success."),
];

fn error(line: usize, message: impl std::fmt::Display) -> String {
    format!("AIL-STE-001 line {line}: {message}")
}

fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn json(value: &AilJsonValue) -> String {
    match value {
        AilJsonValue::String(value) => json_string(value),
        AilJsonValue::Array(values) => format!(
            "[{}]",
            values.iter().map(json).collect::<Vec<_>>().join(",")
        ),
        AilJsonValue::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!("{}:{}", json_string(key), json(value)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn metadata(program: &AilBytecodeProgram) -> Result<AilJsonValue, String> {
    let mut header = program.clone();
    header.actions.clear();
    let AilJsonValue::Object(mut data) = parse_ail_json(&render_ail_bytecode(&header))? else {
        return Err(error(2, "invalid internal metadata"));
    };
    data.remove("actions");
    data.remove("kind");
    if !program.external_bindings_metadata_present {
        data.remove("external_bindings");
    }
    Ok(AilJsonValue::Object(data))
}

fn parse_metadata(text: &str) -> Result<AilBytecodeProgram, String> {
    let original = parse_ail_json(text)?;
    let AilJsonValue::Object(mut data) = original.clone() else {
        return Err("metadata must be an object".to_string());
    };
    if data.contains_key("actions") || data.contains_key("kind") {
        return Err("metadata must not contain actions or an artifact kind".to_string());
    }
    data.insert(
        "kind".to_string(),
        AilJsonValue::String("AIL-Bytecode".to_string()),
    );
    data.insert("actions".to_string(), AilJsonValue::Array(Vec::new()));
    let program = parse_ail_bytecode(&json(&AilJsonValue::Object(data)))?;
    if metadata(&program)? != original {
        return Err("metadata has missing, unknown, or duplicate records".to_string());
    }
    Ok(program)
}

/// Render only known executable instructions. Every operand is declared as data.
pub fn render_ail_ste(program: &AilBytecodeProgram) -> Result<String, String> {
    let diagnostics = verify_ail_bytecode(program);
    if !diagnostics.is_empty() {
        return Err(error(1, diagnostics.join("\n")));
    }
    let mut terms = BTreeMap::new();
    for (key, action) in &program.actions {
        if key != &action.name {
            return Err(error(1, "action key differs from its name"));
        }
        terms.insert(action.name.clone(), String::new());
        for instruction in &action.instructions {
            for value in instruction.operands.values() {
                terms.insert(value.clone(), String::new());
            }
        }
    }
    let mut lines = vec![
        STE_HEADER.to_string(),
        format!("Metadata {}", json(&metadata(program)?)),
    ];
    for (index, (value, name)) in terms.iter_mut().enumerate() {
        *name = format!("t{index}");
        lines.push(format!("Data {name} = {}", json_string(value)));
    }
    for action in program.actions.values() {
        lines.push(format!("Procedure {}.", terms[&action.name]));
        for instruction in &action.instructions {
            let (_, template) = TEMPLATES
                .iter()
                .find(|(op, _)| *op == instruction.opcode)
                .ok_or_else(|| error(1, format!("no STE grammar for {}", instruction.opcode)))?;
            let mut sentence = template.to_string();
            let required = ail_bytecode_required_operands(&instruction.opcode).unwrap();
            let mut attributes = Vec::new();
            for (key, value) in &instruction.operands {
                if !required.contains(&key.as_str()) {
                    return Err(error(
                        1,
                        format!("unknown operand {key} for {}", instruction.opcode),
                    ));
                }
                let placeholder = format!("{{{key}}}");
                if sentence.contains(&placeholder) {
                    sentence = sentence.replace(&placeholder, &terms[value]);
                } else {
                    attributes.push(format!("Attribute {key} = {}", terms[value]));
                }
            }
            lines.push(sentence);
            lines.extend(attributes);
        }
        lines.push("End procedure.".to_string());
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn resolve(terms: &BTreeMap<String, String>, name: &str) -> Result<String, String> {
    terms
        .get(name)
        .cloned()
        .ok_or_else(|| format!("undefined data identifier '{name}'"))
}

fn instruction(
    sentence: &str,
    terms: &BTreeMap<String, String>,
) -> Result<AilBytecodeInstruction, String> {
    for (opcode, template) in TEMPLATES {
        let mut remaining = sentence;
        let mut pattern = *template;
        let mut operands = BTreeMap::new();
        let mut matched = true;
        while let Some((prefix, tail)) = pattern.split_once('{') {
            let Some(rest) = remaining.strip_prefix(prefix) else {
                matched = false;
                break;
            };
            let (key, suffix) = tail.split_once('}').unwrap();
            let end = rest
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            let name = &rest[..end];
            if !identifier(name) {
                matched = false;
                break;
            }
            operands.insert(key.to_string(), name.to_string());
            remaining = &rest[end..];
            pattern = suffix;
        }
        if matched && remaining == pattern {
            for value in operands.values_mut() {
                *value = resolve(terms, value)?;
            }
            return Ok(AilBytecodeInstruction {
                opcode: opcode.to_string(),
                operands,
            });
        }
    }
    Err("unknown instruction; use an exact AIL-STE sentence template".to_string())
}

/// Parse every line and verify control-flow references before returning bytecode.
pub fn parse_ail_ste(text: &str) -> Result<AilBytecodeProgram, String> {
    let mut lines = text.lines().enumerate();
    if lines.next().map(|(_, line)| line) != Some(STE_HEADER) {
        return Err(error(1, "expected AIL-STE 1"));
    }
    let Some((_, header)) = lines.next() else {
        return Err(error(2, "missing metadata"));
    };
    let mut program = parse_metadata(
        header
            .strip_prefix("Metadata ")
            .ok_or_else(|| error(2, "expected metadata"))?,
    )
    .map_err(|e| error(2, e))?;
    let mut terms = BTreeMap::new();
    let mut current: Option<AilBytecodeAction> = None;
    let mut started = false;
    let mut last_line = 2;
    for (index, raw) in lines {
        let line_number = index + 1;
        last_line = line_number;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let result: Result<(), String> = (|| {
            if let Some(data) = line.strip_prefix("Data ") {
                if started {
                    return Err("data declarations must precede procedures".to_string());
                }
                let (name, value) = data
                    .split_once(" = ")
                    .ok_or("expected Data identifier = JSON-string")?;
                if !identifier(name) {
                    return Err("invalid data identifier".to_string());
                }
                let AilJsonValue::String(value) = parse_ail_json(value)? else {
                    return Err("technical data must be a JSON string".to_string());
                };
                if terms.insert(name.to_string(), value).is_some() {
                    return Err(format!("duplicate data identifier '{name}'"));
                }
            } else if let Some(name) = line
                .strip_prefix("Procedure ")
                .and_then(|s| s.strip_suffix('.'))
            {
                if current.is_some() {
                    return Err("close the previous procedure first".to_string());
                }
                let name = resolve(&terms, name)?;
                if program.actions.contains_key(&name) {
                    return Err(format!("duplicate procedure '{name}'"));
                }
                if name.is_empty() {
                    return Err("procedure name must not be empty".to_string());
                }
                current = Some(AilBytecodeAction {
                    name,
                    instructions: Vec::new(),
                });
                started = true;
            } else if line == "End procedure." {
                let action = current.take().ok_or("no procedure to close")?;
                if action.instructions.is_empty() {
                    return Err("procedure must contain an instruction".to_string());
                }
                program.actions.insert(action.name.clone(), action);
            } else if let Some(attribute) = line.strip_prefix("Attribute ") {
                let (key, value) = attribute
                    .split_once(" = ")
                    .ok_or("expected Attribute operand = identifier")?;
                let instruction = current
                    .as_mut()
                    .and_then(|a| a.instructions.last_mut())
                    .ok_or("attribute needs an instruction")?;
                let required = ail_bytecode_required_operands(&instruction.opcode).unwrap();
                if !required.contains(&key) {
                    return Err(format!("unknown operand '{key}'"));
                }
                if instruction
                    .operands
                    .insert(key.to_string(), resolve(&terms, value)?)
                    .is_some()
                {
                    return Err(format!("duplicate operand '{key}'"));
                }
            } else {
                let action = current.as_mut().ok_or("instruction needs a procedure")?;
                action.instructions.push(instruction(line, &terms)?);
            }
            Ok(())
        })();
        result.map_err(|e| error(line_number, e))?;
    }
    if current.is_some() {
        return Err(error(last_line, "missing End procedure."));
    }
    let diagnostics = verify_ail_bytecode(&program);
    if !diagnostics.is_empty() {
        return Err(error(last_line, diagnostics.join("\n")));
    }
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_are_short_unique_and_bind_known_operands() {
        let mut sentences = std::collections::BTreeSet::new();
        for (opcode, template) in TEMPLATES {
            assert!(template.split_whitespace().count() <= 20, "{template}");
            assert!(sentences.insert(template), "{template}");
            let required = ail_bytecode_required_operands(opcode).unwrap();
            for part in template.split('{').skip(1) {
                assert!(
                    required.contains(&part.split_once('}').unwrap().0),
                    "{template}"
                );
            }
        }
    }

    #[test]
    fn every_instruction_template_round_trips_all_operands() {
        let mut program = parse_ail_ste(include_str!("../examples/ste/counter.ail-ste")).unwrap();
        program.type_schemas.insert(
            "Box<Int>".into(),
            BTreeMap::from([(
                "Wrap".into(),
                BTreeMap::from([("value".into(), "Int".into())]),
            )]),
        );
        let action = program.actions.get_mut("IncrementCounter").unwrap();
        action.instructions = TEMPLATES
            .iter()
            .filter(|(opcode, _)| {
                !matches!(
                    *opcode,
                    "INTEGER_FUNCTION"
                        | "CALL_INT"
                        | "RETURN_INT"
                        | "VALUE_FUNCTION"
                        | "CALL_VALUE"
                        | "RETURN_VALUE"
                )
            })
            .map(|(opcode, _)| {
                let operands = ail_bytecode_required_operands(opcode)
                    .unwrap()
                    .iter()
                    .map(|key| {
                        let value = match (*opcode, *key) {
                            ("VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET", "type") => "Box<Int>",
                            ("VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET", "variant") => "Wrap",
                            ("VALUE_CONSTRUCT", "arguments") => "value=i:7",
                            ("VALUE_GET", "field") => "value",
                            ("VALUE_CONSTRUCT", "key") | ("VALUE_TEST" | "VALUE_GET", "source") => {
                                "data.box"
                            }
                            ("VALUE_TEST", "key") => "data.flag",
                            ("VALUE_GET", "key") => "data.number",
                            ("RAISE", "category") => "BYTES",
                            ("TRY_BEGIN", "errors") => "any",
                            ("TRY_BEGIN", "key") => "data.error",
                            ("LABEL", "name") | (_, "label") => "loop_label",
                            ("CALL_ACTION" | "REPEAT_ACTION", "target") => "IncrementCounter",
                            (_, "delta" | "count") => "1",
                            ("BRANCH_INT", "left" | "right") => "i:1",
                            ("EVAL_BOOL" | "BRANCH_BOOL", "expression") => "b:true",
                            ("BRANCH_FLOAT", "left" | "right") => "d:1e0",
                            ("BRANCH_FLOAT", "comparison") => "==",
                            ("EVAL_FLOAT", "expression") => "d:1e0 f:sum.a +",
                            (
                                "EVAL_BOOL" | "EVAL_FLOAT" | "INT_TO_FLOAT" | "FLOAT_TO_INT",
                                "source" | "key",
                            ) => "sum.a",
                            ("BRANCH_INT", "comparison") => "==",
                            ("EVAL_INT", "expression") => "i:1 f:sum.a +",
                            ("JSON_PARSE" | "JSON_FORMAT" | "JSON_GET" | "JSON_SET", "type") => {
                                "Text"
                            }
                            (
                                "JSON_PARSE" | "JSON_FORMAT" | "JSON_GET" | "JSON_COUNT"
                                | "JSON_KIND" | "JSON_KEYS" | "JSON_SET" | "JSON_REMOVE",
                                _,
                            ) => "data.text",
                            (
                                "DIRECTORY_LIST_NAMES"
                                | "DIRECTORY_READ_TEXT"
                                | "DIRECTORY_WRITE_TEXT",
                                "resource",
                            ) => "workspace",
                            (
                                "DIRECTORY_LIST_NAMES"
                                | "DIRECTORY_READ_TEXT"
                                | "DIRECTORY_WRITE_TEXT",
                                _,
                            ) => "data.text",
                            ("PROCESS_RUN", "resource") => "tool",
                            ("PROCESS_RUN", "arguments") => "data.arguments",
                            ("PROCESS_RUN", "input") => "data.input",
                            ("PROCESS_RUN", "timeout_ms") => "data.timeout",
                            ("PROCESS_RUN", "stdout") => "data.stdout",
                            ("PROCESS_RUN", "stderr") => "data.stderr",
                            ("PROCESS_RUN", "status") => "data.status",
                            (
                                "FILE_READ_BYTES" | "FILE_WRITE_BYTES" | "FILE_READ_TEXT"
                                | "FILE_WRITE_TEXT",
                                "resource",
                            ) => "input",
                            (
                                "FILE_READ_BYTES" | "FILE_WRITE_BYTES" | "FILE_READ_TEXT"
                                | "FILE_WRITE_TEXT" | "STDIN_READ_TEXT" | "STDOUT_WRITE_TEXT"
                                | "STDERR_WRITE_TEXT",
                                _,
                            ) => "data.text",
                            ("BYTES_GET" | "BYTES_SET", "index" | "value")
                            | ("BYTES_SLICE", "start" | "end") => "i:1",
                            (
                                "BYTES_LENGTH" | "BYTES_GET" | "BYTES_SLICE" | "BYTES_CONCAT"
                                | "BYTES_SET" | "BYTES_FROM_UTF8" | "BYTES_TO_UTF8"
                                | "BYTES_FROM_HEX" | "BYTES_TO_HEX" | "BYTES_COMPARE",
                                _,
                            ) => "data.bytes",
                            ("ASSIGN_LITERAL", "type") => "Text",
                            ("ASSIGN_LITERAL", "value") => "\"Hello, world!\"",
                            ("ASSIGN_LITERAL", "key") => "data.text",
                            ("EVAL_INT" | "OUTPUT_FIELD", "key") => "sum.total",
                            (
                                "LIST_EMPTY" | "LIST_LENGTH" | "LIST_GET" | "LIST_APPEND"
                                | "LIST_SET" | "LIST_CONCAT" | "LIST_SLICE" | "LIST_REVERSE"
                                | "LIST_SORT",
                                "type",
                            ) => "Int",
                            ("LIST_SORT", "order") => "ascending",
                            ("LIST_SLICE", "start" | "end") => "i:1 f:sum.a +",
                            ("LIST_GET" | "LIST_SET", "index")
                            | ("LIST_APPEND" | "LIST_SET", "value") => "i:1 f:sum.a +",
                            (
                                "LIST_EMPTY" | "LIST_LENGTH" | "LIST_GET" | "LIST_APPEND"
                                | "LIST_SET" | "LIST_CONCAT" | "LIST_SLICE" | "LIST_REVERSE"
                                | "LIST_SORT",
                                _,
                            ) => "list.items",
                            ("TEXT_SLICE", "start" | "end") => "i:1 f:sum.a +",
                            (
                                "TEXT_JOIN" | "TEXT_LENGTH" | "TEXT_SLICE" | "TEXT_FIND"
                                | "TEXT_SPLIT" | "TEXT_JOIN_LIST" | "TEXT_REPLACE" | "TEXT_TRIM",
                                _,
                            ) => "text.value",
                            _ => "technical data \"with quotes\"\nAnd multiple sentences. 🦀",
                        };
                        (key.to_string(), value.to_string())
                    })
                    .collect();
                AilBytecodeInstruction {
                    opcode: opcode.to_string(),
                    operands,
                }
            })
            .collect();
        program
            .actions
            .get_mut("IncrementCounter")
            .unwrap()
            .instructions
            .push(AilBytecodeInstruction {
                opcode: "CALL_INT".into(),
                operands: BTreeMap::from([
                    ("target".into(), "identity".into()),
                    ("arguments".into(), "n=i:5".into()),
                    ("key".into(), "out.result".into()),
                ]),
            });
        program.actions.insert(
            "identity".into(),
            AilBytecodeAction {
                name: "identity".into(),
                instructions: vec![
                    AilBytecodeInstruction {
                        opcode: "INTEGER_FUNCTION".into(),
                        operands: BTreeMap::from([("parameters".into(), "n".into())]),
                    },
                    AilBytecodeInstruction {
                        opcode: "RETURN_INT".into(),
                        operands: BTreeMap::from([("expression".into(), "f:local.n".into())]),
                    },
                ],
            },
        );
        program
            .actions
            .get_mut("IncrementCounter")
            .unwrap()
            .instructions
            .push(AilBytecodeInstruction {
                opcode: "CALL_VALUE".into(),
                operands: BTreeMap::from([
                    ("target".into(), "textIdentity".into()),
                    ("arguments".into(), "text=f:data.text".into()),
                    ("key".into(), "out.text".into()),
                ]),
            });
        program.actions.insert(
            "textIdentity".into(),
            AilBytecodeAction {
                name: "textIdentity".into(),
                instructions: vec![
                    AilBytecodeInstruction {
                        opcode: "VALUE_FUNCTION".into(),
                        operands: BTreeMap::from([
                            ("parameters".into(), "text:Text".into()),
                            ("locals".into(), String::new()),
                            ("result".into(), "Text".into()),
                        ]),
                    },
                    AilBytecodeInstruction {
                        opcode: "RETURN_VALUE".into(),
                        operands: BTreeMap::from([
                            ("expression".into(), "f:local.text".into()),
                            ("type".into(), "Text".into()),
                        ]),
                    },
                ],
            },
        );
        assert_eq!(
            parse_ail_ste(&render_ail_ste(&program).unwrap()).unwrap(),
            program
        );
    }
}
