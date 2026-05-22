use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::imports::{
    filter_document_to_exports, merge_documents, qualify_imported_document_for_alias,
    resolve_import_path, strip_import_section,
};
use crate::rif_model::{
    Application, CollectionDefinition, EndpointDefinition, EndpointErrorDefinition, EnumDefinition,
    ExportDefinition, FailureCase, FieldDefinition, Guarantee, Intent, OperationCall,
    OperationDefinition, OutputValue, Requirement, RifDocument, StateTransition, Step, Thing,
    ThingDefinition, TriggerDefinition, UnresolvedQuestion,
};

pub fn parse_rif_file(path: impl AsRef<Path>) -> Result<RifDocument, String> {
    let path = path.as_ref();
    parse_rif_file_with_imports(path, &mut Vec::new())
}

fn parse_rif_file_with_imports(
    path: &Path,
    stack: &mut Vec<PathBuf>,
) -> Result<RifDocument, String> {
    let resolved_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if stack.contains(&resolved_path) {
        return Err(format!(
            "circular import detected at {}",
            resolved_path.display()
        ));
    }
    stack.push(resolved_path.clone());

    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let (imports, cleaned_text) = strip_import_section(&text);
    let mut document = parse_rif_text(&cleaned_text)?;
    document.source_path = Some(resolved_path.display().to_string());

    for import_path in imports {
        let resolved_import = resolve_import_path(path, &import_path.path)?;
        let imported = parse_rif_file_with_imports(&resolved_import, stack)?;
        let imported = filter_document_to_exports(imported)?;
        let imported = qualify_imported_document_for_alias(imported, import_path.alias.as_deref());
        merge_documents(
            &mut document,
            imported,
            &resolved_import.display().to_string(),
        )?;
    }

    stack.pop();
    Ok(document)
}

pub fn parse_rif_text(text: &str) -> Result<RifDocument, String> {
    let mut application = Application::default();
    let mut current_intent: Option<Intent> = None;
    let mut intents = Vec::new();
    let mut section: Option<String> = None;
    let mut current_step: Option<usize> = None;
    let mut current_failure: Option<usize> = None;
    let mut current_guarantee: Option<usize> = None;
    let mut current_question: Option<usize> = None;
    let mut current_enum: Option<String> = None;
    let mut current_thing: Option<String> = None;
    let mut current_operation: Option<String> = None;
    let mut current_collection: Option<String> = None;
    let mut endpoint_state = EndpointParseState::default();
    let mut current_trigger: Option<usize> = None;
    let mut in_trigger_payload = false;
    let mut in_trigger_bindings = false;
    let mut in_trigger_requires = false;
    let lines = dedent_lines(text);

    for raw_line in &lines {
        let line = raw_line.trim_end();
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with("```") {
            continue;
        }

        if let Some(name) = stripped.strip_prefix("app ") {
            application.name = Some(name.trim().to_string());
            section = None;
            continue;
        }

        if let Some(name) = stripped.strip_prefix("module ") {
            application.module = Some(name.trim().to_string());
            section = None;
            continue;
        }

        if let Some(name) = stripped.strip_prefix("intent ") {
            if let Some(intent) = current_intent.take() {
                intents.push(intent);
            }
            current_intent = Some(Intent::new(name.trim()));
            section = None;
            current_step = None;
            current_failure = None;
            current_guarantee = None;
            current_question = None;
            current_enum = None;
            current_thing = None;
            current_operation = None;
            current_collection = None;
            endpoint_state.reset();
            current_trigger = None;
            in_trigger_payload = false;
            in_trigger_bindings = false;
            in_trigger_requires = false;
            continue;
        }

        if is_section_header(line) {
            section = Some(stripped.trim_end_matches(':').to_ascii_lowercase());
            current_step = None;
            current_failure = None;
            current_guarantee = None;
            current_question = None;
            current_enum = None;
            current_thing = None;
            current_operation = None;
            current_collection = None;
            endpoint_state.reset();
            current_trigger = None;
            in_trigger_payload = false;
            in_trigger_bindings = false;
            in_trigger_requires = false;
            continue;
        }

        if section.as_deref() == Some("types") {
            parse_type_line(&mut application, &mut current_enum, stripped);
            continue;
        }

        if section.as_deref() == Some("exports") {
            parse_export_line(&mut application, stripped);
            continue;
        }

        if section.as_deref() == Some("things") {
            parse_thing_line(&mut application, &mut current_thing, stripped);
            continue;
        }

        if section.as_deref() == Some("operations") {
            parse_operation_line(&mut application, &mut current_operation, stripped);
            continue;
        }

        if section.as_deref() == Some("collections") {
            parse_collection_line(&mut application, &mut current_collection, stripped);
            continue;
        }

        if section.as_deref() == Some("endpoints") {
            parse_endpoint_line(&mut application, &mut endpoint_state, stripped);
            continue;
        }

        if section.as_deref() == Some("triggers") {
            parse_trigger_line(
                &mut application,
                &mut current_trigger,
                &mut in_trigger_payload,
                &mut in_trigger_bindings,
                &mut in_trigger_requires,
                stripped,
            );
            continue;
        }

        let Some(intent) = current_intent.as_mut() else {
            continue;
        };

        match section.as_deref() {
            Some("subject") | Some("inputs") => {
                if let Some((name, type_name)) = parse_declaration(stripped) {
                    let thing = Thing {
                        name: name.clone(),
                        is_secret: is_secret_type(&type_name),
                        type_name,
                    };
                    if section.as_deref() == Some("subject") {
                        intent.subjects.insert(name, thing);
                    } else {
                        intent.inputs.insert(name, thing);
                    }
                }
            }
            Some("requires") => {
                intent.requires.push(Requirement {
                    text: stripped.to_string(),
                });
            }
            Some("state transition") => {
                if let Some(transition) = parse_state_transition(stripped) {
                    intent.state_transitions.push(transition);
                }
            }
            Some("steps") => {
                if let Some(schedule) = stripped.strip_prefix("schedule:") {
                    intent.step_schedule = schedule.trim().to_ascii_lowercase();
                } else if let Some((number, title)) = parse_step_header(stripped) {
                    intent.steps.push(Step::new(number, title));
                    current_step = Some(intent.steps.len() - 1);
                } else if let Some(index) = current_step {
                    parse_step_property(&mut intent.steps[index], stripped);
                }
            }
            Some("failure behavior") => {
                if stripped.starts_with("if ") && stripped.ends_with(':') {
                    let condition = stripped
                        .trim_start_matches("if ")
                        .trim_end_matches(':')
                        .trim()
                        .to_string();
                    intent.failure_handlers.push(FailureCase::new(condition));
                    current_failure = Some(intent.failure_handlers.len() - 1);
                } else if let Some(index) = current_failure {
                    let handler = &mut intent.failure_handlers[index];
                    handler.actions.push(stripped.to_string());
                    if let Some(stop) = stripped.strip_prefix("stop with ") {
                        handler.stop_failure = Some(stop.trim().to_string());
                    }
                    if let Some(ignored) = stripped.strip_prefix("ignore ") {
                        handler.ignored_failures.extend(split_targets(ignored));
                    }
                }
            }
            Some("guarantees") => {
                if stripped.ends_with(':') {
                    intent
                        .guarantees
                        .push(Guarantee::new(stripped.trim_end_matches(':').trim()));
                    current_guarantee = Some(intent.guarantees.len() - 1);
                } else {
                    if current_guarantee.is_none() {
                        intent.guarantees.push(Guarantee {
                            conditions: Vec::new(),
                            statements: Vec::new(),
                        });
                        current_guarantee = Some(intent.guarantees.len() - 1);
                    }
                    if let Some(index) = current_guarantee {
                        intent.guarantees[index]
                            .statements
                            .push(stripped.to_string());
                    }
                }
            }
            Some("unresolved questions") => {
                if let Some(question) = stripped.strip_prefix("- ") {
                    intent.unresolved_questions.push(UnresolvedQuestion {
                        text: question.trim().to_string(),
                    });
                    current_question = Some(intent.unresolved_questions.len() - 1);
                } else if let Some(index) = current_question {
                    let question = &mut intent.unresolved_questions[index];
                    if !question.text.is_empty() {
                        question.text.push('\n');
                    }
                    question.text.push_str(stripped);
                }
            }
            Some("returns") => {
                if let Some((name, source)) = stripped.split_once(':') {
                    intent.returns.push(crate::rif_model::ReturnValue {
                        name: name.trim().to_string(),
                        source: source.trim().to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    if let Some(intent) = current_intent.take() {
        intents.push(intent);
    }

    let first_intent = intents
        .first()
        .cloned()
        .ok_or_else(|| "RIF document must contain an intent line".to_string())?;

    Ok(RifDocument {
        intent: first_intent,
        intents,
        application,
        source_path: None,
    })
}

fn is_section_header(line: &str) -> bool {
    !line.starts_with([' ', '\t']) && line.trim().ends_with(':')
}

fn parse_declaration(text: &str) -> Option<(String, String)> {
    let (name, type_name) = text.split_once(':')?;
    let name = name.trim();
    if !is_identifier(name) {
        return None;
    }
    Some((name.to_string(), type_name.trim().to_string()))
}

fn parse_state_transition(text: &str) -> Option<StateTransition> {
    let (field_path, states) = text.split_once(':')?;
    let (from_state, to_state) = states.split_once("->")?;
    Some(StateTransition {
        field_path: field_path.trim().to_string(),
        from_state: from_state.trim().to_string(),
        to_state: to_state.trim().to_string(),
    })
}

fn parse_export_line(application: &mut Application, text: &str) {
    let text = text.strip_prefix("export ").unwrap_or(text).trim();
    let Some((kind, name)) = text.split_once(' ') else {
        return;
    };
    let kind = kind.trim().to_ascii_lowercase();
    let name = name.trim();
    if kind.is_empty() || name.is_empty() {
        return;
    }
    application.exports.push(ExportDefinition {
        kind: normalize_export_kind(&kind),
        name: name.to_string(),
    });
}

fn normalize_export_kind(kind: &str) -> String {
    match kind {
        "type" | "types" => "enum".to_string(),
        other => other.to_string(),
    }
}

fn parse_step_header(text: &str) -> Option<(usize, String)> {
    let (number, title) = text.split_once(". ")?;
    let number = number.parse::<usize>().ok()?;
    Some((number, title.trim().to_string()))
}

fn parse_step_property(step: &mut Step, text: &str) {
    if let Some(expression) = text.strip_prefix("call:") {
        step.call = Some(parse_call(expression.trim()));
    } else if let Some(expression) = text.strip_prefix("otherwise call:") {
        step.otherwise_call = Some(parse_call(expression.trim()));
    } else if let Some(name) = text.strip_prefix("invoke:") {
        step.invoke = Some(parse_invocation(name.trim()));
    } else if let Some(name) = text.strip_prefix("otherwise invoke:") {
        step.otherwise_invoke = Some(parse_invocation(name.trim()));
    } else if let Some(names) = text.strip_prefix("parallel invoke:") {
        step.parallel_invokes.extend(
            split_targets(names)
                .into_iter()
                .map(|name| parse_invocation(&name)),
        );
    } else if let Some(names) = text.strip_prefix("otherwise parallel invoke:") {
        step.otherwise_parallel_invokes.extend(
            split_targets(names)
                .into_iter()
                .map(|name| parse_invocation(&name)),
        );
    } else if let Some(guard) = text.strip_prefix("when:") {
        step.guard = Some(guard.trim().to_string());
    } else if let Some(condition) = text.strip_prefix("repeat while:") {
        step.repeat_while = Some(condition.trim().to_string());
    } else if let Some(condition) = text.strip_prefix("repeat until:") {
        step.repeat_until = Some(condition.trim().to_string());
    } else if let Some(statement) = text.strip_prefix("set:") {
        step.set_statements.push(statement.trim().to_string());
    } else if let Some(statement) = text.strip_prefix("otherwise set:") {
        step.otherwise_set_statements
            .push(statement.trim().to_string());
    } else if let Some(statement) = text.strip_prefix("append:") {
        step.append_statements.push(statement.trim().to_string());
    } else if let Some(statement) = text.strip_prefix("otherwise append:") {
        step.otherwise_append_statements
            .push(statement.trim().to_string());
    } else if let Some(statement) = text.strip_prefix("compute:") {
        step.compute_statements.push(statement.trim().to_string());
    } else if let Some(statement) = text.strip_prefix("otherwise compute:") {
        step.otherwise_compute_statements
            .push(statement.trim().to_string());
    } else if let Some(statement) = text.strip_prefix("delete:") {
        step.delete_statements.push(statement.trim().to_string());
    } else if let Some(statement) = text.strip_prefix("otherwise delete:") {
        step.otherwise_delete_statements
            .push(statement.trim().to_string());
    } else if let Some(statement) = text.strip_prefix("for each:") {
        let statement = statement.trim();
        if let Some((source, item)) = statement.split_once(" as ") {
            step.iterate_over = Some(source.trim().to_string());
            step.iteration_item = Some(item.trim().to_string());
        }
    } else if let Some(payload) = text.strip_prefix("output:") {
        let payload = payload.trim();
        if let Some((name, type_name)) = parse_declaration(payload) {
            step.outputs.insert(
                name.clone(),
                OutputValue {
                    name,
                    is_secret: is_secret_type(&type_name),
                    type_name,
                },
            );
        } else {
            step.outputs.insert(
                payload.to_string(),
                OutputValue {
                    name: payload.to_string(),
                    type_name: String::new(),
                    is_secret: false,
                },
            );
        }
    } else if let Some(targets) = text.strip_prefix("reads:") {
        step.reads.extend(split_targets(targets));
    } else if let Some(targets) = text.strip_prefix("changes:") {
        step.changes.extend(split_targets(targets));
    } else if let Some(targets) = text.strip_prefix("external call:") {
        step.external_calls.extend(split_targets(targets));
    } else if let Some(failures) = text.strip_prefix("may fail with:") {
        step.may_fail.extend(split_targets(failures));
    } else if let Some(compensation) = text.strip_prefix("compensation:") {
        step.compensation = Some(compensation.trim().to_string());
    } else if let Some(failures) = text.strip_prefix("ignore failure:") {
        step.ignored_failures.extend(split_targets(failures));
    } else if let Some(failures) = text.strip_prefix("ignore failures:") {
        step.ignored_failures.extend(split_targets(failures));
    } else {
        step.raw_lines.push(text.to_string());
    }
}

fn parse_type_line(application: &mut Application, current_enum: &mut Option<String>, text: &str) {
    if let Some(declaration) = text.strip_prefix("enum ") {
        let declaration = declaration.trim();
        let (name, values) = declaration
            .split_once(':')
            .map(|(name, values)| (name.trim(), split_targets(values)))
            .unwrap_or((declaration, Vec::new()));
        if name.is_empty() {
            return;
        }
        application
            .enums
            .entry(name.to_string())
            .or_insert_with(|| EnumDefinition::new(name))
            .values
            .extend(values);
        *current_enum = Some(name.to_string());
        return;
    }

    if let Some(value) = text.strip_prefix("value ")
        && let Some(enum_name) = current_enum.as_ref()
        && let Some(enum_definition) = application.enums.get_mut(enum_name)
    {
        let value = value.trim();
        if is_identifier(value) {
            enum_definition.values.push(value.to_string());
        }
    }
}

fn parse_thing_line(application: &mut Application, current_thing: &mut Option<String>, text: &str) {
    if let Some(name) = text.strip_prefix("thing ") {
        let name = name.trim().to_string();
        application
            .things
            .entry(name.clone())
            .or_insert_with(|| ThingDefinition::new(&name));
        *current_thing = Some(name);
        return;
    }

    if let Some(field) = text.strip_prefix("field ")
        && let Some(thing_name) = current_thing.as_ref()
        && let Some((name, type_name)) = parse_declaration(field.trim())
        && let Some(thing) = application.things.get_mut(thing_name)
    {
        thing.fields.insert(
            name.clone(),
            FieldDefinition {
                name,
                is_secret: is_secret_type(&type_name),
                type_name,
            },
        );
    }
}

fn parse_operation_line(
    application: &mut Application,
    current_operation: &mut Option<String>,
    text: &str,
) {
    if let Some(signature) = text.strip_prefix("operation ") {
        let operation = parse_operation_signature(signature.trim());
        let name = operation.name.clone();
        application.operations.insert(name.clone(), operation);
        *current_operation = Some(name);
        return;
    }

    let Some(operation_name) = current_operation.as_ref() else {
        return;
    };
    let Some(operation) = application.operations.get_mut(operation_name) else {
        return;
    };

    if let Some(targets) = text.strip_prefix("reads:") {
        operation.reads.extend(split_targets(targets));
    } else if let Some(targets) = text.strip_prefix("changes:") {
        operation.changes.extend(split_targets(targets));
    } else if let Some(targets) = text.strip_prefix("external call:") {
        operation.external_calls.extend(split_targets(targets));
    } else if let Some(failures) = text.strip_prefix("may fail with:") {
        operation.may_fail.extend(split_targets(failures));
    } else if let Some(payload) = text.strip_prefix("output:")
        && let Some((name, type_name)) = parse_declaration(payload.trim())
    {
        operation.outputs.push(OutputValue {
            name,
            is_secret: is_secret_type(&type_name),
            type_name,
        });
    }
}

fn parse_collection_line(
    application: &mut Application,
    current_collection: &mut Option<String>,
    text: &str,
) {
    if let Some(declaration) = text.strip_prefix("collection ") {
        let declaration = declaration.trim();
        let Some((name, type_name)) = declaration.split_once(':') else {
            return;
        };
        let name = name.trim();
        let type_name = type_name.trim();
        if name.is_empty() || type_name.is_empty() {
            return;
        }
        application
            .collections
            .insert(name.to_string(), CollectionDefinition::new(name, type_name));
        *current_collection = Some(name.to_string());
        return;
    }

    if let Some(unique_fields) = text.strip_prefix("unique:")
        && let Some(collection_name) = current_collection.as_ref()
        && let Some(collection) = application.collections.get_mut(collection_name)
    {
        for field in split_targets(unique_fields) {
            if !collection.unique_fields.contains(&field) {
                collection.unique_fields.push(field);
            }
        }
    }
}

#[derive(Default)]
struct EndpointParseState {
    current: Option<usize>,
    in_request: bool,
    in_bindings: bool,
    in_requires: bool,
    in_response: bool,
    in_error: bool,
    current_error_case: Option<String>,
}

impl EndpointParseState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn enter_bindings(&mut self) {
        self.in_request = false;
        self.in_bindings = true;
        self.in_requires = false;
        self.in_response = false;
        self.in_error = false;
        self.current_error_case = None;
    }

    fn enter_requires(&mut self) {
        self.in_request = false;
        self.in_bindings = false;
        self.in_requires = true;
        self.in_response = false;
        self.in_error = false;
        self.current_error_case = None;
    }

    fn enter_response(&mut self) {
        self.in_request = false;
        self.in_bindings = false;
        self.in_requires = false;
        self.in_response = true;
        self.in_error = false;
        self.current_error_case = None;
    }

    fn enter_error(&mut self, error_case: Option<String>) {
        self.in_request = false;
        self.in_bindings = false;
        self.in_requires = false;
        self.in_response = false;
        self.in_error = true;
        self.current_error_case = error_case;
    }

    fn enter_request(&mut self) {
        self.in_request = true;
        self.in_bindings = false;
        self.in_requires = false;
        self.in_response = false;
        self.in_error = false;
        self.current_error_case = None;
    }
}

fn parse_endpoint_line(application: &mut Application, state: &mut EndpointParseState, text: &str) {
    if let Some((method, path, target)) = parse_endpoint_header(text) {
        application.endpoints.push(EndpointDefinition {
            method: method.to_ascii_uppercase(),
            path,
            target: normalize_name(&target),
            request_fields: BTreeMap::new(),
            requires: Vec::new(),
            bindings: BTreeMap::new(),
            response_status: None,
            response_fields: BTreeMap::new(),
            responses: BTreeMap::new(),
            error_status: None,
            error_fields: BTreeMap::new(),
            error_responses: BTreeMap::new(),
            error_cases: BTreeMap::new(),
        });
        state.reset();
        state.current = Some(application.endpoints.len() - 1);
        return;
    }

    if text == "request:" {
        state.enter_request();
        return;
    }

    if text == "bind:" {
        state.enter_bindings();
        return;
    }

    if text == "requires:" {
        state.enter_requires();
        return;
    }

    if text == "respond:" {
        state.enter_response();
        return;
    }

    if text == "error:" || (text.starts_with("error ") && text.ends_with(':')) {
        let error_case = text
            .strip_prefix("error ")
            .and_then(|name| name.strip_suffix(':'))
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToString::to_string);
        state.enter_error(error_case);
        return;
    }

    let Some(index) = state.current.as_ref() else {
        return;
    };
    let Some(endpoint) = application.endpoints.get_mut(*index) else {
        return;
    };

    if state.in_request
        && let Some((name, type_name)) = text.split_once(':')
    {
        endpoint
            .request_fields
            .insert(name.trim().to_string(), type_name.trim().to_string());
        return;
    }

    if state.in_bindings
        && let Some((target, source)) = text.split_once('=')
    {
        endpoint
            .bindings
            .insert(target.trim().to_string(), source.trim().to_string());
        return;
    }

    if state.in_requires {
        endpoint.requires.push(text.to_string());
        return;
    }

    if state.in_response
        && let Some(status) = text.strip_prefix("status:")
        && looks_like_http_status(status.trim())
    {
        let status = status.trim();
        if !status.is_empty() {
            endpoint.response_status = Some(status.to_string());
        }
        return;
    }

    if state.in_response
        && let Some((name, source)) = text.split_once('=')
    {
        endpoint
            .responses
            .insert(name.trim().to_string(), source.trim().to_string());
        return;
    }

    if state.in_response
        && let Some((name, type_name)) = text.split_once(':')
    {
        endpoint
            .response_fields
            .insert(name.trim().to_string(), type_name.trim().to_string());
        return;
    }

    if state.in_error
        && let Some((name, source)) = text.split_once('=')
    {
        if let Some(error_name) = state.current_error_case.as_ref() {
            endpoint
                .error_cases
                .entry(error_name.clone())
                .or_insert_with(EndpointErrorDefinition::default)
                .responses
                .insert(name.trim().to_string(), source.trim().to_string());
        } else {
            endpoint
                .error_responses
                .insert(name.trim().to_string(), source.trim().to_string());
        }
        return;
    }

    if state.in_error
        && let Some(status) = text.strip_prefix("status:")
        && looks_like_http_status(status.trim())
    {
        let status = status.trim();
        if !status.is_empty() {
            if let Some(error_name) = state.current_error_case.as_ref() {
                endpoint
                    .error_cases
                    .entry(error_name.clone())
                    .or_insert_with(EndpointErrorDefinition::default)
                    .status = Some(status.to_string());
            } else {
                endpoint.error_status = Some(status.to_string());
            }
        }
        return;
    }

    if state.in_error
        && let Some((name, type_name)) = text.split_once(':')
    {
        if let Some(error_name) = state.current_error_case.as_ref() {
            endpoint
                .error_cases
                .entry(error_name.clone())
                .or_insert_with(EndpointErrorDefinition::default)
                .response_fields
                .insert(name.trim().to_string(), type_name.trim().to_string());
        } else {
            endpoint
                .error_fields
                .insert(name.trim().to_string(), type_name.trim().to_string());
        }
        return;
    }

    if let Some(requirement) = text.strip_prefix("requires:") {
        let requirement = requirement.trim();
        if !requirement.is_empty() {
            endpoint.requires.push(requirement.to_string());
        }
    }
}

fn parse_trigger_line(
    application: &mut Application,
    current_trigger: &mut Option<usize>,
    in_trigger_payload: &mut bool,
    in_trigger_bindings: &mut bool,
    in_trigger_requires: &mut bool,
    text: &str,
) {
    if let Some((name, target)) = parse_trigger_header(text) {
        application.triggers.push(TriggerDefinition {
            name,
            target: normalize_name(&target),
            schedule: None,
            queue: None,
            payload_fields: BTreeMap::new(),
            requires: Vec::new(),
            bindings: BTreeMap::new(),
        });
        *current_trigger = Some(application.triggers.len() - 1);
        *in_trigger_payload = false;
        *in_trigger_bindings = false;
        *in_trigger_requires = false;
        return;
    }

    if text == "payload:" {
        *in_trigger_payload = true;
        *in_trigger_bindings = false;
        *in_trigger_requires = false;
        return;
    }

    if text == "bind:" {
        *in_trigger_payload = false;
        *in_trigger_bindings = true;
        *in_trigger_requires = false;
        return;
    }

    if text == "requires:" {
        *in_trigger_payload = false;
        *in_trigger_bindings = false;
        *in_trigger_requires = true;
        return;
    }

    let Some(index) = current_trigger.as_ref() else {
        return;
    };
    let Some(trigger) = application.triggers.get_mut(*index) else {
        return;
    };

    if *in_trigger_payload && let Some((name, type_name)) = text.split_once(':') {
        trigger
            .payload_fields
            .insert(name.trim().to_string(), type_name.trim().to_string());
        return;
    }

    if *in_trigger_bindings && let Some((target, source)) = text.split_once('=') {
        trigger
            .bindings
            .insert(target.trim().to_string(), source.trim().to_string());
        return;
    }

    if *in_trigger_requires {
        trigger.requires.push(text.to_string());
        return;
    }

    if let Some(schedule) = text.strip_prefix("schedule:") {
        let schedule = schedule.trim();
        if !schedule.is_empty() {
            trigger.schedule = Some(schedule.to_string());
        }
        return;
    }

    if let Some(queue) = text.strip_prefix("queue:") {
        let queue = queue.trim();
        if !queue.is_empty() {
            trigger.queue = Some(queue.to_string());
        }
        return;
    }

    if let Some(requirement) = text.strip_prefix("requires:") {
        let requirement = requirement.trim();
        if !requirement.is_empty() {
            trigger.requires.push(requirement.to_string());
        }
    }
}

fn parse_endpoint_header(text: &str) -> Option<(String, String, String)> {
    let text = text.strip_prefix("endpoint ")?;
    let (head, target) = text.split_once("->")?;
    let mut parts = head.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    Some((method, path, target.trim().to_string()))
}

fn looks_like_http_status(text: &str) -> bool {
    text.split_whitespace()
        .next()
        .and_then(|code| code.parse::<u16>().ok())
        .is_some()
}

fn parse_trigger_header(text: &str) -> Option<(String, String)> {
    let text = text.strip_prefix("trigger ")?;
    let (name, target) = text.split_once("->")?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), target.trim().to_string()))
}

fn parse_operation_signature(signature: &str) -> OperationDefinition {
    let (head, output_text) = signature
        .split_once("->")
        .map(|(head, output)| (head.trim(), Some(output.trim())))
        .unwrap_or((signature.trim(), None));
    let (name, args_text) = if let Some(open) = head.find('(') {
        let close = head.rfind(')').unwrap_or(head.len());
        (head[..open].trim(), &head[open + 1..close])
    } else {
        (head, "")
    };
    let mut operation = OperationDefinition::new(name);
    for arg in split_targets(args_text) {
        if let Some((arg_name, type_name)) = parse_declaration(&arg) {
            operation.input_order.push(arg_name.clone());
            operation.inputs.insert(arg_name, type_name);
        }
    }
    if let Some(output) = output_text
        && !output.is_empty()
        && output != "Unit"
    {
        operation.outputs.push(OutputValue {
            name: "result".to_string(),
            type_name: output.to_string(),
            is_secret: is_secret_type(output),
        });
    }
    operation
}

fn parse_call(expression: &str) -> OperationCall {
    let expression = expression.trim();
    let Some(open) = expression.find('(') else {
        return OperationCall {
            expression: expression.to_string(),
            target: expression.to_string(),
            args: Vec::new(),
        };
    };
    let close = expression.rfind(')').unwrap_or(expression.len());
    let target = expression[..open].trim().to_string();
    let args_text = &expression[open + 1..close];
    OperationCall {
        expression: expression.to_string(),
        target,
        args: split_targets(args_text),
    }
}

fn parse_invocation(text: &str) -> crate::rif_model::InvocationTarget {
    let text = text.trim();
    let Some(open) = text.find('(') else {
        return crate::rif_model::InvocationTarget {
            target: normalize_name(text),
            bindings: BTreeMap::new(),
        };
    };
    let close = text.rfind(')').unwrap_or(text.len());
    let target = normalize_name(text[..open].trim());
    let bindings_text = &text[open + 1..close];
    let mut bindings = BTreeMap::new();
    for binding in split_targets(bindings_text) {
        if let Some((name, value)) = binding.split_once('=') {
            bindings.insert(name.trim().to_string(), value.trim().to_string());
        }
    }
    crate::rif_model::InvocationTarget { target, bindings }
}

fn split_targets(text: &str) -> Vec<String> {
    split_top_level_commas(text)
        .into_iter()
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn normalize_name(text: &str) -> String {
    let mut out = String::new();
    let mut uppercase_next = true;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            if uppercase_next {
                out.push(ch.to_ascii_uppercase());
                uppercase_next = false;
            } else {
                out.push(ch);
            }
        } else {
            uppercase_next = true;
        }
    }
    out
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

fn is_secret_type(type_name: &str) -> bool {
    type_name.trim().starts_with("Secret<")
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn dedent_lines(text: &str) -> Vec<String> {
    let raw_lines: Vec<&str> = text.lines().collect();
    let indent = raw_lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.chars()
                .take_while(|ch| *ch == ' ' || *ch == '\t')
                .count()
        })
        .min()
        .unwrap_or(0);

    raw_lines
        .into_iter()
        .map(|line| {
            if line.len() >= indent {
                line[indent..].to_string()
            } else {
                line.to_string()
            }
        })
        .collect()
}
