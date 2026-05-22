use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::imports::{
    filter_document_to_exports, merge_documents, qualify_imported_document_for_alias,
    resolve_import_path, strip_import_section,
};
use crate::rif_model::{
    Application, EnumDefinition, ExportDefinition, FailureCase, FieldDefinition, Guarantee, Intent,
    OperationCall, OperationDefinition, OutputValue, Requirement, RifDocument, StateTransition,
    Step, Thing, ThingDefinition,
};

#[derive(Debug, Clone, Default)]
struct PhraseDefinition {
    name: String,
    means: Option<OperationCall>,
    produces: Option<OutputValue>,
    changes: Vec<String>,
    external_calls: Vec<String>,
    may_fail: Vec<String>,
    compensation: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct RslIntent {
    name: String,
    subjects: BTreeMap<String, Thing>,
    inputs: BTreeMap<String, Thing>,
    requires: Vec<String>,
    state_transition: Option<StateTransition>,
    compact_steps: Vec<String>,
    failure_routes: Vec<(String, String)>,
    guarantees: Vec<String>,
}

pub fn parse_rsl_file(path: impl AsRef<Path>) -> Result<RifDocument, String> {
    let path = path.as_ref();
    parse_rsl_file_with_imports(path, &mut Vec::new())
}

fn parse_rsl_file_with_imports(
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
    let mut document = parse_rsl_text(&cleaned_text)?;
    document.source_path = Some(resolved_path.display().to_string());

    for import_path in imports {
        let resolved_import = resolve_import_path(path, &import_path.path)?;
        let imported = parse_rsl_file_with_imports(&resolved_import, stack)?;
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

pub fn parse_rsl_text(text: &str) -> Result<RifDocument, String> {
    let lines = dedent_lines(text);
    let mut application = Application::default();
    let mut phrases: BTreeMap<String, PhraseDefinition> = BTreeMap::new();
    let mut current_intent: Option<RslIntent> = None;
    let mut current_section: Option<String> = None;
    let mut current_enum: Option<String> = None;
    let mut current_thing: Option<String> = None;
    let mut current_operation: Option<String> = None;
    let mut current_phrase: Option<String> = None;
    let mut current_guarantee: Option<usize> = None;
    let mut current_prose_failure: Option<usize> = None;
    let mut intents = Vec::new();
    let mut errors = Vec::new();

    for raw_line in &lines {
        let line = raw_line.trim_end();
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with("```") {
            continue;
        }

        if let Some(name) = stripped.strip_prefix("app ") {
            application.name = Some(name.trim().to_string());
            current_section = None;
            current_phrase = None;
            continue;
        }

        if let Some(name) = stripped.strip_prefix("module ") {
            application.module = Some(name.trim().to_string());
            current_section = None;
            current_phrase = None;
            continue;
        }

        if let Some(name) = stripped.strip_prefix("intent ") {
            if let Some(intent) = current_intent.take() {
                intents.push(intent);
            }
            current_intent = Some(RslIntent {
                name: normalize_name(name.trim()),
                ..Default::default()
            });
            current_section = None;
            current_enum = None;
            current_thing = None;
            current_operation = None;
            current_phrase = None;
            current_guarantee = None;
            current_prose_failure = None;
            continue;
        }

        if is_section_header(line) {
            current_section = Some(stripped.trim_end_matches(':').to_ascii_lowercase());
            current_enum = None;
            current_thing = None;
            current_operation = None;
            current_phrase = None;
            current_guarantee = None;
            current_prose_failure = None;
            continue;
        }

        match current_section.as_deref() {
            Some("types") => {
                parse_type_line(&mut application, &mut current_enum, stripped);
                continue;
            }
            Some("exports") => {
                parse_export_line(&mut application, stripped);
                continue;
            }
            Some("things") => {
                if let Some(intent) = current_intent.as_mut() {
                    if let Some((subject_name, from_state, to_state, condition)) =
                        parse_stateful_action_sentence(stripped)
                    {
                        if let Some(requirement) = condition {
                            intent.requires.push(requirement);
                        }
                        if intent.state_transition.is_none()
                            && let Some(field_path) =
                                state_transition_field_path(intent, &application, &subject_name)
                        {
                            intent.state_transition = Some(StateTransition {
                                field_path,
                                from_state,
                                to_state,
                            });
                        }
                        continue;
                    }
                    if let Some(condition) = stripped
                        .strip_prefix("If ")
                        .and_then(|text| text.strip_suffix(':'))
                        .map(|text| text.trim().to_string())
                    {
                        intent.failure_routes.push((condition, String::new()));
                        current_prose_failure = Some(intent.failure_routes.len() - 1);
                        continue;
                    }
                    if let Some(index) = current_prose_failure
                        && !stripped.ends_with(':')
                    {
                        let route = &mut intent.failure_routes[index];
                        if route.1.is_empty() {
                            route.1 = stripped.to_string();
                        } else {
                            route.1.push(' ');
                            route.1.push_str(stripped);
                        }
                        continue;
                    }
                }
                parse_thing_line(&mut application, &mut current_thing, stripped);
                continue;
            }
            Some("operations") => {
                parse_operation_line(&mut application, &mut current_operation, stripped);
                continue;
            }
            Some("phrases") => {
                parse_phrase_line(&mut phrases, &mut current_phrase, stripped, &mut errors);
                continue;
            }
            _ => {}
        }

        let Some(intent) = current_intent.as_mut() else {
            continue;
        };

        if let Some(step) = stripped.strip_prefix("by ") {
            intent.compact_steps = split_targets(step);
            continue;
        }

        if let Some(route) = stripped.strip_prefix("on ")
            && let Some((condition, target)) = route.split_once("->")
        {
            intent
                .failure_routes
                .push((condition.trim().to_string(), target.trim().to_string()));
            continue;
        }

        if let Some((from_state, to_state)) = parse_compact_transition(stripped) {
            intent.state_transition = Some(StateTransition {
                field_path: String::new(),
                from_state,
                to_state,
            });
            continue;
        }

        if let Some((subject_name, from_state, to_state, condition)) =
            parse_stateful_action_sentence(stripped)
        {
            if let Some(requirement) = condition {
                intent.requires.push(requirement);
            }
            if intent.state_transition.is_none()
                && let Some(field_path) =
                    state_transition_field_path(intent, &application, &subject_name)
            {
                intent.state_transition = Some(StateTransition {
                    field_path,
                    from_state,
                    to_state,
                });
            }
            continue;
        }

        if let Some(condition) = stripped
            .strip_prefix("If ")
            .and_then(|text| text.strip_suffix(':'))
            .map(|text| text.trim().to_string())
        {
            intent.failure_routes.push((condition, String::new()));
            current_prose_failure = Some(intent.failure_routes.len() - 1);
            continue;
        }

        if let Some((name, type_name)) = parse_declaration(stripped) {
            let thing = Thing {
                name: name.clone(),
                is_secret: is_secret_type(&type_name),
                type_name,
            };
            if current_section.as_deref() == Some("subject") {
                intent.subjects.insert(name, thing);
            } else if current_section.as_deref() == Some("inputs") {
                intent.inputs.insert(name, thing);
            } else {
                intent.requires.push(stripped.to_string());
            }
            continue;
        }

        if let Some(index) = current_prose_failure
            && !stripped.ends_with(':')
        {
            let route = &mut intent.failure_routes[index];
            if route.1.is_empty() {
                route.1 = stripped.to_string();
            } else {
                route.1.push(' ');
                route.1.push_str(stripped);
            }
            continue;
        }

        match current_section.as_deref() {
            Some("requires") => intent.requires.push(stripped.to_string()),
            Some("state transition") => {
                if let Some(transition) = parse_state_transition(stripped) {
                    intent.state_transition = Some(transition);
                }
            }
            Some("steps") => {
                // Compact lines are handled above so the section may stay open for prose.
            }
            Some("failure behavior") => {
                if stripped.starts_with("if ") && stripped.ends_with(':') {
                    intent.failure_routes.push((
                        stripped[3..stripped.len() - 1].trim().to_string(),
                        String::new(),
                    ));
                    current_prose_failure = Some(intent.failure_routes.len() - 1);
                }
            }
            Some("guarantees") => {
                if stripped.ends_with(':') {
                    intent
                        .guarantees
                        .push(stripped.trim_end_matches(':').trim().to_string());
                    current_guarantee = Some(intent.guarantees.len() - 1);
                } else if let Some(index) = current_guarantee {
                    intent.guarantees[index].push(' ');
                    intent.guarantees[index].push_str(stripped);
                } else {
                    intent.guarantees.push(stripped.to_string());
                    current_guarantee = Some(intent.guarantees.len() - 1);
                }
            }
            _ => {}
        }
    }

    if let Some(intent) = current_intent.take() {
        intents.push(intent);
    }

    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    let first_intent = intents
        .first()
        .cloned()
        .ok_or_else(|| "RSL document must contain an intent line".to_string())?;

    let mut rendered_application = application.clone();
    let mut document = RifDocument {
        intent: Intent::new(first_intent.name.clone()),
        intents: Vec::new(),
        application: rendered_application.clone(),
        source_path: None,
    };

    for intent in intents {
        document.intents.push(elaborate_intent(
            &mut rendered_application,
            &document.application,
            intent,
            &phrases,
        )?);
    }

    document.application = rendered_application;
    if let Some(intent) = document.intents.first().cloned() {
        document.intent = intent;
    }

    Ok(document)
}

fn elaborate_intent(
    rendered_application: &mut Application,
    application: &Application,
    raw: RslIntent,
    phrases: &BTreeMap<String, PhraseDefinition>,
) -> Result<Intent, String> {
    let mut intent = Intent::new(raw.name);
    intent.subjects = raw.subjects.clone();
    intent.inputs = raw.inputs.clone();
    if intent.subjects.is_empty()
        && let Some((subject_name, type_name)) =
            infer_subject_from_intent_name(&intent.name, application)
    {
        intent.subjects.insert(
            subject_name.clone(),
            Thing {
                name: subject_name,
                type_name,
                is_secret: false,
            },
        );
    }
    intent.requires = raw
        .requires
        .into_iter()
        .map(|text| Requirement { text })
        .collect();

    if let Some(mut transition) = raw.state_transition.clone() {
        if transition.field_path.is_empty() {
            transition.field_path = infer_state_field_path(&intent, application)?;
        }
        intent.state_transitions.push(transition);
    }

    let mut known_values = known_values_for_intent(&intent);
    let mut phrase_steps = Vec::new();
    let mut failures_with_steps = Vec::new();

    for (index, phrase_name) in raw.compact_steps.into_iter().enumerate() {
        let phrase_key = phrase_name.trim().to_ascii_lowercase();
        let Some(phrase) = phrases.get(&phrase_key) else {
            return Err(format!("unknown phrase '{}'", phrase_name.trim()));
        };
        let step = elaborate_phrase_step(
            index + 1,
            &phrase.name,
            phrase,
            &known_values,
            application,
            rendered_application,
        )?;
        for output in step.outputs.values() {
            known_values.insert(output.name.clone(), output.type_name.clone());
        }
        if !step.may_fail.is_empty() {
            failures_with_steps.push((step.title.clone(), step.may_fail.clone()));
        }
        phrase_steps.push(step);
    }

    if let Some(transition) = intent.state_transitions.first() {
        let field_path = transition.field_path.clone();
        let title = format!(
            "Mark {} {}",
            field_path
                .split('.')
                .next()
                .unwrap_or(&field_path)
                .to_ascii_lowercase(),
            transition.to_state.to_ascii_lowercase()
        );
        let mut step = Step::new(phrase_steps.len() + 1, title);
        step.set_statements
            .push(format!("{} = {}", field_path, transition.to_state));
        step.changes.push(field_path.clone());
        phrase_steps.push(step);
    }

    intent.steps = phrase_steps;

    if !raw.failure_routes.is_empty() {
        let failing_steps: Vec<_> = intent
            .steps
            .iter()
            .filter(|step| !step.may_fail.is_empty())
            .collect();
        for (index, (condition, target)) in raw.failure_routes.into_iter().enumerate() {
            let mut handler = FailureCase::new(condition);
            let fallback_failure = failing_steps
                .get(index)
                .and_then(|step| step.may_fail.first())
                .cloned()
                .unwrap_or_else(|| target.clone());
            if target.chars().any(|ch| ch.is_ascii_uppercase()) && !target.contains(' ') {
                handler.stop_failure = Some(target.clone());
            } else {
                if target.contains('=') {
                    if let Some(transition) = intent.state_transitions.first()
                        && let Some(subject_name) = intent.subjects.keys().next()
                    {
                        handler
                            .actions
                            .push(format!("set {} = {}", transition.field_path, target));
                        handler.stop_failure = Some(fallback_failure.clone());
                        if !subject_name.is_empty() && handler.actions.is_empty() {
                            handler
                                .actions
                                .push(format!("set {} = {}", subject_name, target));
                        }
                    }
                } else if !target.is_empty() {
                    handler.actions.push(target.clone());
                }
                if handler.stop_failure.is_none() {
                    handler.stop_failure = Some(fallback_failure.clone());
                }
            }
            if handler.actions.is_empty()
                && let Some(step) = failing_steps.get(index)
                && let Some(transition) = intent.state_transitions.first()
            {
                handler
                    .actions
                    .push(format!("set {} = {}", transition.field_path, target));
                handler.stop_failure = Some(
                    step.may_fail
                        .first()
                        .cloned()
                        .unwrap_or_else(|| target.clone()),
                );
            }
            intent.failure_handlers.push(handler);
        }
    } else {
        for (_, failures) in failures_with_steps {
            for failure in failures {
                let mut handler = FailureCase::new("if this intent fails");
                handler.stop_failure = Some(failure);
                intent.failure_handlers.push(handler);
            }
        }
    }

    intent.guarantees = raw
        .guarantees
        .into_iter()
        .map(|statement| Guarantee {
            conditions: vec!["if this intent succeeds".to_string()],
            statements: vec![statement],
        })
        .collect();

    Ok(intent)
}

fn elaborate_phrase_step(
    number: usize,
    title: &str,
    phrase: &PhraseDefinition,
    known_values: &BTreeMap<String, String>,
    application: &Application,
    rendered_application: &mut Application,
) -> Result<Step, String> {
    let mut step = Step::new(number, sentence_case(title));
    let Some(call) = &phrase.means else {
        return Err(format!("phrase '{}' is missing a means clause", title));
    };

    let rendered_call = call.clone();
    let mut operation = rendered_application
        .operations
        .remove(&rendered_call.target)
        .unwrap_or_else(|| OperationDefinition::new(&rendered_call.target));

    for (index, arg) in rendered_call.args.iter().enumerate() {
        let input_name = format!("arg{}", index + 1);
        let Some(arg_type) = infer_expression_type(arg, known_values, application) else {
            return Err(format!(
                "phrase '{}' has unknown argument reference '{}'",
                title, arg
            ));
        };
        operation.input_order.push(input_name.clone());
        operation.inputs.insert(input_name, arg_type);
        step.reads.push(arg.clone());
    }

    if let Some(output) = &phrase.produces {
        operation.outputs.push(output.clone());
        step.outputs.insert(output.name.clone(), output.clone());
    }
    operation.changes = phrase.changes.clone();
    operation.external_calls = phrase.external_calls.clone();
    operation.may_fail = phrase.may_fail.clone();
    rendered_application
        .operations
        .insert(operation.name.clone(), operation);

    step.call = Some(rendered_call);
    step.changes = phrase.changes.clone();
    step.external_calls = phrase.external_calls.clone();
    step.may_fail = phrase.may_fail.clone();
    step.compensation = phrase.compensation.clone();

    Ok(step)
}

fn infer_expression_type(
    expression: &str,
    known_values: &BTreeMap<String, String>,
    application: &Application,
) -> Option<String> {
    let expression = expression.trim();
    if let Some(value) = known_values.get(expression) {
        return Some(value.clone());
    }
    let (root, field_path) = expression.split_once('.').unwrap_or((expression, ""));
    let root_type = known_values.get(root)?.clone();
    if field_path.is_empty() {
        return Some(root_type);
    }
    let mut current_type = root_type;
    for field_name in field_path.split('.') {
        let thing = application.things.get(&current_type)?;
        let field = thing.fields.get(field_name)?;
        current_type = field.type_name.clone();
    }
    Some(current_type)
}

fn known_values_for_intent(intent: &Intent) -> BTreeMap<String, String> {
    let mut known = BTreeMap::new();
    for (name, thing) in intent.subjects.iter().chain(intent.inputs.iter()) {
        known.insert(name.clone(), thing.type_name.clone());
    }
    known
}

fn infer_state_field_path(intent: &Intent, application: &Application) -> Result<String, String> {
    let Some((subject_name, subject)) = intent.subjects.iter().next() else {
        return Err("compact state transition requires a subject".to_string());
    };
    let Some(thing) = application.things.get(&subject.type_name) else {
        return Err(format!(
            "unknown subject type '{}' for '{}'",
            subject.type_name, subject_name
        ));
    };
    if thing.fields.contains_key("status") {
        return Ok(format!("{subject_name}.status"));
    }
    if let Some(field) = thing.fields.keys().next() {
        return Ok(format!("{subject_name}.{field}"));
    }
    Err(format!(
        "subject '{}' has no fields to use in a compact state transition",
        subject_name
    ))
}

fn infer_subject_from_intent_name(
    intent_name: &str,
    _application: &Application,
) -> Option<(String, String)> {
    let subject_name = pascal_case_tokens(intent_name)
        .into_iter()
        .last()
        .map(|part| part.to_ascii_lowercase())?;
    let candidate_type = subject_name[..1].to_ascii_uppercase() + &subject_name[1..];
    Some((subject_name, candidate_type))
}

fn pascal_case_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut previous_is_lower = false;
    for ch in text.chars() {
        if ch.is_ascii_uppercase() && previous_is_lower && !current.is_empty() {
            tokens.push(current);
            current = String::new();
        }
        if ch.is_ascii_alphanumeric() {
            current.push(ch);
        }
        previous_is_lower = ch.is_ascii_lowercase();
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_phrase_line(
    phrases: &mut BTreeMap<String, PhraseDefinition>,
    current_phrase: &mut Option<String>,
    stripped: &str,
    errors: &mut Vec<String>,
) {
    if let Some(declaration) = stripped.strip_prefix("phrase ") {
        let declaration = declaration.trim_end_matches(':').trim();
        let name = parse_quoted_text(declaration).unwrap_or_else(|| declaration.to_string());
        let key = name.to_ascii_lowercase();
        if phrases.contains_key(&key) {
            errors.push(format!("ambiguous phrase '{}'", name));
            *current_phrase = Some(key);
            return;
        }
        phrases.insert(
            key.clone(),
            PhraseDefinition {
                name,
                ..Default::default()
            },
        );
        *current_phrase = Some(key);
        return;
    }

    let Some(phrase_key) = current_phrase.as_ref() else {
        return;
    };
    let Some(phrase) = phrases.get_mut(phrase_key) else {
        return;
    };

    if let Some(means) = stripped
        .strip_prefix("means ")
        .or_else(|| stripped.strip_prefix("means:"))
    {
        phrase.means = Some(parse_call_expression(means.trim()));
    } else if let Some(produces) = stripped.strip_prefix("produces ") {
        if let Some((name, type_name)) = parse_declaration(produces.trim()) {
            phrase.produces = Some(OutputValue {
                name,
                type_name,
                is_secret: false,
            });
        }
    } else if let Some(changes) = stripped.strip_prefix("changes ") {
        phrase.changes.extend(split_targets(changes));
    } else if let Some(calls) = stripped.strip_prefix("calls ") {
        phrase.external_calls.extend(split_targets(calls));
    } else if let Some(failures) = stripped.strip_prefix("may fail with ") {
        phrase.may_fail.extend(split_targets(failures));
    } else if let Some(compensation) = stripped.strip_prefix("compensation is ") {
        phrase.compensation = Some(compensation.trim().to_string());
    } else if let Some(compensation) = stripped.strip_prefix("compensation:") {
        phrase.compensation = Some(compensation.trim().to_string());
    }
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

    if let Some((thing_name, field_name, type_name)) = parse_thing_has_sentence(text) {
        application
            .things
            .entry(thing_name.clone())
            .or_insert_with(|| ThingDefinition::new(&thing_name))
            .fields
            .insert(
                field_name.clone(),
                FieldDefinition {
                    name: field_name,
                    is_secret: is_secret_type(&type_name),
                    type_name,
                },
            );
        *current_thing = Some(thing_name);
        return;
    }

    if let Some((thing_name, states)) = parse_thing_can_be_sentence(text) {
        application
            .things
            .entry(thing_name.clone())
            .or_insert_with(|| ThingDefinition::new(&thing_name))
            .fields
            .insert(
                "status".to_string(),
                FieldDefinition {
                    name: "status".to_string(),
                    is_secret: false,
                    type_name: format!("State<{}>", states.join(", ")),
                },
            );
        *current_thing = Some(thing_name);
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

fn parse_thing_has_sentence(text: &str) -> Option<(String, String, String)> {
    let sentence = text.trim().trim_end_matches('.');
    let rest = sentence
        .strip_prefix("A ")
        .or_else(|| sentence.strip_prefix("An "))?;
    let (thing_name, property) = rest.split_once(" has ")?;
    let property = property
        .trim()
        .trim_start_matches("a ")
        .trim_start_matches("an ")
        .trim_start_matches("the ");
    let field_name = to_snake_case(property);
    Some((
        thing_name.trim().to_string(),
        field_name,
        "Text".to_string(),
    ))
}

fn parse_thing_can_be_sentence(text: &str) -> Option<(String, Vec<String>)> {
    let sentence = text.trim().trim_end_matches('.');
    let rest = sentence
        .strip_prefix("A ")
        .or_else(|| sentence.strip_prefix("An "))?;
    let (thing_name, states) = rest.split_once(" can be ")?;
    Some((thing_name.trim().to_string(), split_targets(states)))
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
    }
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

fn to_snake_case(text: &str) -> String {
    let mut out = String::new();
    let mut previous_was_separator = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if !out.is_empty() && !previous_was_separator {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push(ch.to_ascii_lowercase());
            }
            previous_was_separator = false;
        } else if !previous_was_separator && !out.is_empty() {
            out.push('_');
            previous_was_separator = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    while out.starts_with('_') {
        out.remove(0);
    }
    out
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

fn parse_compact_transition(text: &str) -> Option<(String, String)> {
    let (from_state, to_state) = text.split_once("->")?;
    Some((from_state.trim().to_string(), to_state.trim().to_string()))
}

fn parse_stateful_action_sentence(text: &str) -> Option<(String, String, String, Option<String>)> {
    let sentence = text.trim().trim_end_matches('.');
    let sentence = sentence
        .strip_prefix("A ")
        .or_else(|| sentence.strip_prefix("An "))?;
    let (subject_phrase, rest) = sentence.split_once(" can be ")?;
    let (from_phrase, thing_name) = subject_phrase.rsplit_once(' ')?;
    let to_and_condition = rest.trim();
    let (to_phrase, condition) = to_and_condition
        .split_once(" when ")
        .map(|(to, condition)| (to.trim(), Some(condition.trim().to_string())))
        .unwrap_or((to_and_condition, None));
    Some((
        thing_name.trim().to_string(),
        sentence_case(from_phrase.trim()),
        sentence_case(to_phrase.trim()),
        condition,
    ))
}

fn parse_call_expression(expression: &str) -> OperationCall {
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

fn split_targets(text: &str) -> Vec<String> {
    split_top_level_commas(text)
        .into_iter()
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToString::to_string)
        .collect()
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

fn parse_declaration(text: &str) -> Option<(String, String)> {
    let (name, type_name) = text.split_once(':')?;
    let name = name.trim();
    if !is_identifier(name) {
        return None;
    }
    Some((name.to_string(), type_name.trim().to_string()))
}

fn parse_quoted_text(text: &str) -> Option<String> {
    let text = text.trim();
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.to_string())
}

fn state_transition_field_path(
    intent: &RslIntent,
    application: &Application,
    subject_name: &str,
) -> Option<String> {
    if let Some(thing) = intent
        .subjects
        .get(subject_name)
        .or_else(|| intent.subjects.values().next())
        && let Some(definition) = application.things.get(&thing.type_name)
        && definition.fields.contains_key("status")
    {
        return Some(format!("{subject_name}.status"));
    }
    intent
        .subjects
        .keys()
        .next()
        .map(|name| format!("{name}.status"))
}

fn is_section_header(line: &str) -> bool {
    matches!(
        line.trim()
            .trim_end_matches(':')
            .to_ascii_lowercase()
            .as_str(),
        "types"
            | "things"
            | "operations"
            | "phrases"
            | "subject"
            | "inputs"
            | "requires"
            | "state transition"
            | "steps"
            | "failure behavior"
            | "guarantees"
    )
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

fn normalize_name(text: &str) -> String {
    let mut out = String::new();
    for part in text
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
    {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars.map(|ch| ch.to_ascii_lowercase()));
        }
    }
    if out.is_empty() {
        text.to_string()
    } else {
        out
    }
}

fn sentence_case(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.extend(chars);
    out
}
