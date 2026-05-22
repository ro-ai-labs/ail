use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::rif_model::{EndpointDefinition, Intent, RifDocument, Step, TriggerDefinition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportSpec {
    pub path: String,
    pub alias: Option<String>,
}

pub(crate) fn strip_import_section(text: &str) -> (Vec<ImportSpec>, String) {
    let mut imports = Vec::new();
    let mut cleaned = Vec::new();
    let mut in_imports = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "imports:" {
            in_imports = true;
            continue;
        }
        if in_imports {
            if trimmed.is_empty() || trimmed.starts_with("```") {
                continue;
            }
            if trimmed.starts_with("app ")
                || trimmed.starts_with("module ")
                || trimmed.starts_with("intent ")
                || (trimmed.ends_with(':') && trimmed != "imports:")
            {
                in_imports = false;
                cleaned.push(line.to_string());
                continue;
            }
            if let Some(import_path) = trimmed.strip_prefix("import ") {
                let import_path = import_path.trim();
                if !import_path.is_empty() {
                    imports.push(parse_import_spec(import_path));
                }
                continue;
            }
            if !line.starts_with(' ') && !line.starts_with('\t') {
                in_imports = false;
                cleaned.push(line.to_string());
                continue;
            }
            imports.push(parse_import_spec(trimmed));
            continue;
        }
        cleaned.push(line.to_string());
    }

    (imports, cleaned.join("\n"))
}

fn parse_import_spec(text: &str) -> ImportSpec {
    if let Some((path, alias)) = text.split_once(" as ") {
        return ImportSpec {
            path: path.trim().to_string(),
            alias: Some(normalize_name(alias.trim())),
        };
    }
    ImportSpec {
        path: text.trim().to_string(),
        alias: None,
    }
}

pub(crate) fn resolve_import_path(base_path: &Path, import_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(import_path);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    };
    fs::canonicalize(&candidate).map_err(|error| {
        format!(
            "failed to resolve import '{}': {error}",
            candidate.display()
        )
    })
}

pub(crate) fn merge_documents(
    document: &mut RifDocument,
    imported: RifDocument,
    source_label: &str,
) -> Result<(), String> {
    merge_application_name(document, &imported);
    merge_application_module(document, &imported);
    merge_declared_maps(
        &mut document.application.enums,
        imported.application.enums,
        source_label,
        "enum",
    )?;
    merge_declared_maps(
        &mut document.application.things,
        imported.application.things,
        source_label,
        "thing",
    )?;
    merge_declared_maps(
        &mut document.application.operations,
        imported.application.operations,
        source_label,
        "operation",
    )?;
    merge_declared_maps(
        &mut document.application.collections,
        imported.application.collections,
        source_label,
        "collection",
    )?;
    merge_endpoints(
        &mut document.application.endpoints,
        imported.application.endpoints,
        source_label,
    )?;
    merge_triggers(
        &mut document.application.triggers,
        imported.application.triggers,
        source_label,
    )?;
    for intent in imported.intents {
        if document
            .intents
            .iter()
            .any(|existing| existing.name == intent.name)
        {
            return Err(format!(
                "conflicting imported intent '{}' from {}",
                intent.name, source_label
            ));
        }
        document.intents.push(intent);
    }
    if let Some(imported_first) = document.intents.first().cloned() {
        document.intent = imported_first;
    }
    Ok(())
}

pub(crate) fn filter_document_to_exports(mut document: RifDocument) -> Result<RifDocument, String> {
    if document.application.exports.is_empty() {
        return Ok(document);
    }

    let mut exports_by_kind: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for export in &document.application.exports {
        if !export_exists(&document, &export.kind, &export.name) {
            return Err(format!(
                "exported {} '{}' is not declared",
                export.kind, export.name
            ));
        }
        exports_by_kind
            .entry(export.kind.clone())
            .or_default()
            .insert(export.name.clone());
    }

    document
        .application
        .enums
        .retain(|name, _| is_exported(&exports_by_kind, "enum", name));
    document
        .application
        .things
        .retain(|name, _| is_exported(&exports_by_kind, "thing", name));
    document
        .application
        .operations
        .retain(|name, _| is_exported(&exports_by_kind, "operation", name));
    document
        .application
        .collections
        .retain(|name, _| is_exported(&exports_by_kind, "collection", name));
    document.application.endpoints.retain(|endpoint| {
        is_exported(
            &exports_by_kind,
            "endpoint",
            &endpoint_export_name(endpoint),
        )
    });
    document
        .application
        .triggers
        .retain(|trigger| is_exported(&exports_by_kind, "trigger", &trigger.name));
    document
        .intents
        .retain(|intent| is_exported(&exports_by_kind, "intent", &intent.name));
    if let Some(first_intent) = document.intents.first().cloned() {
        document.intent = first_intent;
    }
    document.application.exports.clear();
    Ok(document)
}

fn qualify_imported_document(mut document: RifDocument, alias: &str) -> RifDocument {
    let mut names = BTreeMap::new();
    document.application.enums = qualify_declared_map(
        document.application.enums,
        alias,
        &mut names,
        |definition, qualified| definition.name = qualified.clone(),
    );
    document.application.things = qualify_declared_map(
        document.application.things,
        alias,
        &mut names,
        |definition, qualified| definition.name = qualified.clone(),
    );
    document.application.operations = qualify_declared_map(
        document.application.operations,
        alias,
        &mut names,
        |definition, qualified| definition.name = qualified.clone(),
    );
    document.application.collections = qualify_declared_map(
        document.application.collections,
        alias,
        &mut names,
        |definition, qualified| definition.name = qualified.clone(),
    );

    document.application.endpoints = document
        .application
        .endpoints
        .into_iter()
        .map(|mut endpoint| {
            endpoint.path = qualify_path(alias, &endpoint.path);
            endpoint.target = qualify_name(alias, &endpoint.target);
            endpoint
        })
        .collect();
    document.application.triggers = document
        .application
        .triggers
        .into_iter()
        .map(|mut trigger| {
            let original_name = trigger.name.clone();
            trigger.name = qualify_name(alias, &trigger.name);
            names.insert(original_name, trigger.name.clone());
            trigger.target = qualify_name(alias, &trigger.target);
            trigger
        })
        .collect();

    let mut qualified_intents = Vec::new();
    for mut intent in document.intents {
        let original_name = intent.name.clone();
        intent.name = qualify_name(alias, &intent.name);
        names.insert(original_name, intent.name.clone());
        qualified_intents.push(intent);
    }
    document.intents = qualified_intents;

    qualify_document_references(&mut document, &names);
    if let Some(first_intent) = document.intents.first().cloned() {
        document.intent = first_intent;
    }
    document
}

fn qualify_declared_map<T, F>(
    values: BTreeMap<String, T>,
    alias: &str,
    names: &mut BTreeMap<String, String>,
    mut rename_definition: F,
) -> BTreeMap<String, T>
where
    F: FnMut(&mut T, &String),
{
    let mut qualified = BTreeMap::new();
    for (name, mut value) in values {
        let qualified_name = qualify_name(alias, &name);
        rename_definition(&mut value, &qualified_name);
        names.insert(name, qualified_name.clone());
        qualified.insert(qualified_name, value);
    }
    qualified
}

pub(crate) fn export_exists(document: &RifDocument, kind: &str, name: &str) -> bool {
    match kind {
        "enum" => document.application.enums.contains_key(name),
        "thing" => document.application.things.contains_key(name),
        "operation" => document.application.operations.contains_key(name),
        "collection" => document.application.collections.contains_key(name),
        "endpoint" => document
            .application
            .endpoints
            .iter()
            .any(|endpoint| endpoint_export_name(endpoint) == name),
        "trigger" => document
            .application
            .triggers
            .iter()
            .any(|trigger| trigger.name == name),
        "intent" => document.intents.iter().any(|intent| intent.name == name),
        _ => false,
    }
}

fn is_exported(
    exports_by_kind: &BTreeMap<String, BTreeSet<String>>,
    kind: &str,
    name: &str,
) -> bool {
    exports_by_kind
        .get(kind)
        .is_some_and(|names| names.contains(name))
}

pub(crate) fn endpoint_export_name(endpoint: &EndpointDefinition) -> String {
    format!("{} {}", endpoint.method.to_ascii_uppercase(), endpoint.path)
}

fn qualify_document_references(document: &mut RifDocument, names: &BTreeMap<String, String>) {
    let replacements = replacement_pairs(names);
    for thing in document.application.things.values_mut() {
        for field in thing.fields.values_mut() {
            field.type_name = rename_text(&field.type_name, &replacements);
        }
    }
    for operation in document.application.operations.values_mut() {
        for value in operation.inputs.values_mut() {
            *value = rename_text(value, &replacements);
        }
        for output in &mut operation.outputs {
            output.type_name = rename_text(&output.type_name, &replacements);
        }
        operation.reads = operation
            .reads
            .iter()
            .map(|value| rename_text(value, &replacements))
            .collect();
        operation.changes = operation
            .changes
            .iter()
            .map(|value| rename_text(value, &replacements))
            .collect();
        operation.external_calls = operation
            .external_calls
            .iter()
            .map(|value| rename_text(value, &replacements))
            .collect();
        operation.may_fail = operation
            .may_fail
            .iter()
            .map(|value| rename_text(value, &replacements))
            .collect();
    }
    for collection in document.application.collections.values_mut() {
        collection.type_name = rename_text(&collection.type_name, &replacements);
    }
    for endpoint in &mut document.application.endpoints {
        endpoint.request_fields = endpoint
            .request_fields
            .iter()
            .map(|(name, type_name)| (name.clone(), rename_text(type_name, &replacements)))
            .collect();
        endpoint.requires = endpoint
            .requires
            .iter()
            .map(|value| rename_text(value, &replacements))
            .collect();
        endpoint.bindings = endpoint
            .bindings
            .iter()
            .map(|(target, source)| (target.clone(), rename_text(source, &replacements)))
            .collect();
        endpoint.response_fields = endpoint
            .response_fields
            .iter()
            .map(|(name, type_name)| (name.clone(), rename_text(type_name, &replacements)))
            .collect();
        endpoint.responses = endpoint
            .responses
            .iter()
            .map(|(name, source)| (name.clone(), rename_text(source, &replacements)))
            .collect();
        endpoint.error_fields = endpoint
            .error_fields
            .iter()
            .map(|(name, type_name)| (name.clone(), rename_text(type_name, &replacements)))
            .collect();
        endpoint.error_responses = endpoint
            .error_responses
            .iter()
            .map(|(name, source)| (name.clone(), rename_text(source, &replacements)))
            .collect();
        for error in endpoint.error_cases.values_mut() {
            error.response_fields = error
                .response_fields
                .iter()
                .map(|(name, type_name)| (name.clone(), rename_text(type_name, &replacements)))
                .collect();
            error.responses = error
                .responses
                .iter()
                .map(|(name, source)| (name.clone(), rename_text(source, &replacements)))
                .collect();
        }
    }
    for trigger in &mut document.application.triggers {
        trigger.payload_fields = trigger
            .payload_fields
            .iter()
            .map(|(name, type_name)| (name.clone(), rename_text(type_name, &replacements)))
            .collect();
        trigger.requires = trigger
            .requires
            .iter()
            .map(|value| rename_text(value, &replacements))
            .collect();
        trigger.bindings = trigger
            .bindings
            .iter()
            .map(|(target, source)| (target.clone(), rename_text(source, &replacements)))
            .collect();
    }
    for intent in &mut document.intents {
        rename_intent_references(intent, &replacements);
    }
}

fn rename_intent_references(intent: &mut Intent, replacements: &[(String, String)]) {
    intent.subjects = std::mem::take(&mut intent.subjects)
        .into_iter()
        .map(|(name, mut thing)| {
            thing.type_name = rename_text(&thing.type_name, replacements);
            (name, thing)
        })
        .collect();
    intent.inputs = std::mem::take(&mut intent.inputs)
        .into_iter()
        .map(|(name, mut thing)| {
            thing.type_name = rename_text(&thing.type_name, replacements);
            (name, thing)
        })
        .collect();
    intent.requires = intent
        .requires
        .iter()
        .map(|requirement| crate::rif_model::Requirement {
            text: rename_text(&requirement.text, replacements),
        })
        .collect();
    intent.state_transitions = intent
        .state_transitions
        .iter()
        .map(|transition| crate::rif_model::StateTransition {
            field_path: transition.field_path.clone(),
            from_state: rename_text(&transition.from_state, replacements),
            to_state: rename_text(&transition.to_state, replacements),
        })
        .collect();
    for step in &mut intent.steps {
        rename_step(step, replacements);
    }
    for handler in &mut intent.failure_handlers {
        handler.condition = rename_text(&handler.condition, replacements);
        handler.actions = handler
            .actions
            .iter()
            .map(|value| rename_text(value, replacements))
            .collect();
        handler.stop_failure = handler
            .stop_failure
            .as_ref()
            .map(|value| rename_text(value, replacements));
        handler.ignored_failures = handler
            .ignored_failures
            .iter()
            .map(|value| rename_text(value, replacements))
            .collect();
    }
    for guarantee in &mut intent.guarantees {
        guarantee.conditions = guarantee
            .conditions
            .iter()
            .map(|value| rename_text(value, replacements))
            .collect();
        guarantee.statements = guarantee
            .statements
            .iter()
            .map(|value| rename_text(value, replacements))
            .collect();
    }
    intent.unresolved_questions = intent
        .unresolved_questions
        .iter()
        .map(|question| crate::rif_model::UnresolvedQuestion {
            text: rename_text(&question.text, replacements),
        })
        .collect();
    intent.returns = intent
        .returns
        .iter()
        .map(|value| crate::rif_model::ReturnValue {
            name: value.name.clone(),
            source: rename_text(&value.source, replacements),
        })
        .collect();
}

fn rename_step(step: &mut Step, replacements: &[(String, String)]) {
    if let Some(call) = &mut step.call {
        call.target = rename_text(&call.target, replacements);
        call.expression = rename_text(&call.expression, replacements);
        call.args = call
            .args
            .iter()
            .map(|value| rename_text(value, replacements))
            .collect();
    }
    if let Some(call) = &mut step.otherwise_call {
        call.target = rename_text(&call.target, replacements);
        call.expression = rename_text(&call.expression, replacements);
        call.args = call
            .args
            .iter()
            .map(|value| rename_text(value, replacements))
            .collect();
    }
    if let Some(invoke) = &mut step.invoke {
        invoke.target = rename_text(&invoke.target, replacements);
        invoke.bindings = invoke
            .bindings
            .iter()
            .map(|(target, source)| (target.clone(), rename_text(source, replacements)))
            .collect();
    }
    if let Some(invoke) = &mut step.otherwise_invoke {
        invoke.target = rename_text(&invoke.target, replacements);
        invoke.bindings = invoke
            .bindings
            .iter()
            .map(|(target, source)| (target.clone(), rename_text(source, replacements)))
            .collect();
    }
    for target in &mut step.parallel_invokes {
        target.target = rename_text(&target.target, replacements);
        target.bindings = target
            .bindings
            .iter()
            .map(|(key, value)| (key.clone(), rename_text(value, replacements)))
            .collect();
    }
    for target in &mut step.otherwise_parallel_invokes {
        target.target = rename_text(&target.target, replacements);
        target.bindings = target
            .bindings
            .iter()
            .map(|(key, value)| (key.clone(), rename_text(value, replacements)))
            .collect();
    }
    step.guard = step
        .guard
        .as_ref()
        .map(|value| rename_text(value, replacements));
    step.repeat_while = step
        .repeat_while
        .as_ref()
        .map(|value| rename_text(value, replacements));
    step.repeat_until = step
        .repeat_until
        .as_ref()
        .map(|value| rename_text(value, replacements));
    step.set_statements = step
        .set_statements
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.otherwise_set_statements = step
        .otherwise_set_statements
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.compute_statements = step
        .compute_statements
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.otherwise_compute_statements = step
        .otherwise_compute_statements
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.delete_statements = step
        .delete_statements
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.otherwise_delete_statements = step
        .otherwise_delete_statements
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.outputs.values_mut().for_each(|output| {
        output.type_name = rename_text(&output.type_name, replacements);
    });
    step.reads = step
        .reads
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.changes = step
        .changes
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.external_calls = step
        .external_calls
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.may_fail = step
        .may_fail
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.compensation = step
        .compensation
        .as_ref()
        .map(|value| rename_text(value, replacements));
    step.ignored_failures = step
        .ignored_failures
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
    step.raw_lines = step
        .raw_lines
        .iter()
        .map(|value| rename_text(value, replacements))
        .collect();
}

fn replacement_pairs(names: &BTreeMap<String, String>) -> Vec<(String, String)> {
    let mut pairs: Vec<_> = names
        .iter()
        .map(|(name, qualified)| (name.clone(), qualified.clone()))
        .collect();
    pairs.sort_by(|left, right| right.0.len().cmp(&left.0.len()));
    pairs
}

fn rename_text(text: &str, replacements: &[(String, String)]) -> String {
    let mut out = String::new();
    let mut token = String::new();
    for ch in text.chars() {
        if ch == '_' || ch.is_ascii_alphanumeric() {
            token.push(ch);
        } else {
            if !token.is_empty() {
                out.push_str(&rename_token(&token, replacements));
                token.clear();
            }
            out.push(ch);
        }
    }
    if !token.is_empty() {
        out.push_str(&rename_token(&token, replacements));
    }
    out
}

fn rename_token(token: &str, replacements: &[(String, String)]) -> String {
    replacements
        .iter()
        .find_map(|(name, qualified)| (name == token).then(|| qualified.clone()))
        .unwrap_or_else(|| token.to_string())
}

fn qualify_name(alias: &str, name: &str) -> String {
    format!("{alias}.{name}")
}

fn qualify_path(alias: &str, path: &str) -> String {
    if path.starts_with('/') {
        format!("/{alias}{}", path)
    } else {
        format!("{alias}/{path}")
    }
}

pub(crate) fn qualify_imported_document_for_alias(
    document: RifDocument,
    alias: Option<&str>,
) -> RifDocument {
    if let Some(alias) = alias {
        qualify_imported_document(document, alias)
    } else {
        document
    }
}

fn merge_application_name(document: &mut RifDocument, imported: &RifDocument) {
    if document.application.name.is_none() {
        document.application.name = imported.application.name.clone();
    }
}

fn merge_application_module(document: &mut RifDocument, imported: &RifDocument) {
    if document.application.module.is_none() {
        document.application.module = imported.application.module.clone();
    }
}

fn merge_declared_maps<T: Clone>(
    target: &mut BTreeMap<String, T>,
    imported: BTreeMap<String, T>,
    source_label: &str,
    kind: &str,
) -> Result<(), String> {
    for (name, value) in imported {
        match target.entry(name.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
            Entry::Occupied(_) => {
                return Err(format!(
                    "conflicting imported {} '{}' from {}",
                    kind, name, source_label
                ));
            }
        }
    }
    Ok(())
}

fn merge_endpoints(
    target: &mut Vec<EndpointDefinition>,
    imported: Vec<EndpointDefinition>,
    source_label: &str,
) -> Result<(), String> {
    for endpoint in imported {
        if target.iter().any(|existing| {
            existing.method.eq_ignore_ascii_case(&endpoint.method) && existing.path == endpoint.path
        }) {
            return Err(format!(
                "conflicting imported endpoint '{} {}' from {}",
                endpoint.method, endpoint.path, source_label
            ));
        }
        target.push(endpoint);
    }
    Ok(())
}

fn merge_triggers(
    target: &mut Vec<TriggerDefinition>,
    imported: Vec<TriggerDefinition>,
    source_label: &str,
) -> Result<(), String> {
    for trigger in imported {
        if target.iter().any(|existing| existing.name == trigger.name) {
            return Err(format!(
                "conflicting imported trigger '{}' from {}",
                trigger.name, source_label
            ));
        }
        target.push(trigger);
    }
    Ok(())
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
