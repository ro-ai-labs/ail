use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::Diagnostic;
use crate::expression;
use crate::graph_builder::build_program;
use crate::imports::export_exists;
use crate::predicate;
use crate::rif_model::{
    EndpointDefinition, FailureCase, Intent, InvocationTarget, OperationCall, OutputValue,
    RifDocument, Step, TriggerDefinition,
};

pub fn check_document(document: &RifDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_graph_integrity(document));
    diagnostics.extend(check_application_declarations(document));
    diagnostics.extend(check_exports(document));
    diagnostics.extend(check_endpoints(document));
    diagnostics.extend(check_triggers(document));
    for intent in &document.intents {
        diagnostics.extend(check_invoke_targets(document, &intent.steps));
        diagnostics.extend(check_call_targets(document, &intent.steps));
        diagnostics.extend(check_invocation_bindings(document, intent));
        diagnostics.extend(check_operation_call_signatures(document, intent));
        diagnostics.extend(check_output_types(document, &intent.steps));
        diagnostics.extend(check_failure_completeness(
            document,
            &intent.steps,
            &intent.failure_handlers,
        ));
        diagnostics.extend(check_unordered_change_conflicts(
            &intent.steps,
            &intent.step_schedule,
        ));
        diagnostics.extend(check_requirements(document, intent));
        diagnostics.extend(check_set_assignments(document, intent));
        diagnostics.extend(check_guards(document, intent));
        diagnostics.extend(check_repeats(document, intent));
        diagnostics.extend(check_compute_expressions(document, intent));
        diagnostics.extend(check_append_assignments(document, intent));
        diagnostics.extend(check_returns(document, intent));
    }
    diagnostics.extend(check_secret_flows(document));
    diagnostics.extend(check_guarantees(document));
    diagnostics.extend(check_unresolved_questions(document));
    for intent in &document.intents {
        diagnostics.extend(check_compensations(&intent.steps));
    }

    diagnostics
}

fn check_exports(document: &RifDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for export in &document.application.exports {
        if !is_known_export_kind(&export.kind) {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_EXPORT_KIND",
                format!(
                    "Export '{}' uses unknown declaration kind '{}'.",
                    export.name, export.kind
                ),
            ));
            continue;
        }
        if !export_exists(document, &export.kind, &export.name) {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_EXPORT",
                format!(
                    "Exported {} '{}' is not declared.",
                    export.kind, export.name
                ),
            ));
        }
    }
    diagnostics
}

fn is_known_export_kind(kind: &str) -> bool {
    matches!(
        kind,
        "enum" | "thing" | "operation" | "collection" | "endpoint" | "trigger" | "intent"
    )
}

fn check_graph_integrity(document: &RifDocument) -> Vec<Diagnostic> {
    build_program(document)
        .graph
        .validate_edge_references()
        .into_iter()
        .map(|message| Diagnostic::error("EIGL_GRAPH_EDGE_REFERENCE", message))
        .collect()
}

fn check_application_declarations(document: &RifDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_types = known_types(document);
    for enum_definition in document.application.enums.values() {
        if enum_definition.values.is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_EMPTY_ENUM",
                format!(
                    "Enum '{}' must declare at least one value.",
                    enum_definition.name
                ),
            ));
        }
    }
    for thing in document.application.things.values() {
        for field in thing.fields.values() {
            for type_name in referenced_types(&field.type_name) {
                if !known_types.contains(&type_name) {
                    diagnostics.push(Diagnostic::error(
                        "EIGL_UNKNOWN_TYPE",
                        format!(
                            "Thing '{}' field '{}' uses unknown type '{}'.",
                            thing.name, field.name, type_name
                        ),
                    ));
                }
            }
        }
    }
    for operation in document.application.operations.values() {
        for type_name in operation.inputs.values() {
            for referenced in referenced_types(type_name) {
                if !known_types.contains(&referenced) {
                    diagnostics.push(Diagnostic::error(
                        "EIGL_UNKNOWN_TYPE",
                        format!(
                            "Operation '{}' input uses unknown type '{}'.",
                            operation.name, referenced
                        ),
                    ));
                }
            }
        }
        for output in &operation.outputs {
            for referenced in referenced_types(&output.type_name) {
                if !known_types.contains(&referenced) {
                    diagnostics.push(Diagnostic::error(
                        "EIGL_UNKNOWN_TYPE",
                        format!(
                            "Operation '{}' output uses unknown type '{}'.",
                            operation.name, referenced
                        ),
                    ));
                }
            }
        }
    }
    for collection in document.application.collections.values() {
        if collection.name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_COLLECTION_NAME_REQUIRED",
                "Collections must declare a name.",
            ));
        }
        for referenced in referenced_types(&collection.type_name) {
            if !known_types.contains(&referenced) {
                diagnostics.push(Diagnostic::error(
                    "EIGL_UNKNOWN_TYPE",
                    format!(
                        "Collection '{}' uses unknown type '{}'.",
                        collection.name, referenced
                    ),
                ));
            }
        }
        for unique_field in &collection.unique_fields {
            if field_type(document, &collection.type_name, unique_field).is_none() {
                diagnostics.push(Diagnostic::error(
                    "EIGL_UNKNOWN_COLLECTION_UNIQUE_FIELD",
                    format!(
                        "Collection '{}' declares unique field '{}' but type '{}' has no such field.",
                        collection.name, unique_field, collection.type_name
                    ),
                ));
            }
        }
        if document.application.things.contains_key(&collection.name)
            || document
                .application
                .operations
                .contains_key(&collection.name)
        {
            diagnostics.push(Diagnostic::error(
                "EIGL_COLLECTION_NAME_CONFLICT",
                format!(
                    "Collection '{}' conflicts with another application declaration.",
                    collection.name
                ),
            ));
        }
    }
    diagnostics
}

fn check_endpoints(document: &RifDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_intents: BTreeSet<_> = document
        .intents
        .iter()
        .map(|intent| intent.name.clone())
        .collect();
    for endpoint in &document.application.endpoints {
        if endpoint.method.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_METHOD_REQUIRED",
                "Endpoints must declare an HTTP method.",
            ));
        }
        if endpoint.path.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_PATH_REQUIRED",
                "Endpoints must declare a request path.",
            ));
        }
        if !known_intents.contains(&endpoint.target) {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_ENDPOINT_INTENT",
                format!(
                    "Endpoint '{} {}' targets unknown intent '{}'.",
                    endpoint.method, endpoint.path, endpoint.target
                ),
            ));
        }
        diagnostics.extend(check_endpoint_request_fields(document, endpoint));
        diagnostics.extend(check_endpoint_requirements(document, endpoint));
        diagnostics.extend(check_endpoint_bindings(document, endpoint));
        diagnostics.extend(check_endpoint_responses(document, endpoint));
    }
    diagnostics
}

fn check_endpoint_request_fields(
    document: &RifDocument,
    endpoint: &EndpointDefinition,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_types = known_types(document);
    for (name, type_name) in &endpoint.request_fields {
        if name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_REQUEST_NAME_REQUIRED",
                format!(
                    "Endpoint '{} {}' declares a request field with no name.",
                    endpoint.method, endpoint.path
                ),
            ));
        }
        if type_name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_REQUEST_TYPE_REQUIRED",
                format!(
                    "Endpoint '{} {}' request field '{}' requires a type.",
                    endpoint.method, endpoint.path, name
                ),
            ));
            continue;
        }
        for referenced in referenced_types(type_name) {
            if !known_types.contains(&referenced) {
                diagnostics.push(Diagnostic::error(
                    "EIGL_UNKNOWN_ENDPOINT_REQUEST_TYPE",
                    format!(
                        "Endpoint '{} {}' request field '{}' uses unknown type '{}'.",
                        endpoint.method, endpoint.path, name, referenced
                    ),
                ));
            }
        }
    }
    diagnostics
}

fn check_triggers(document: &RifDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_intents: BTreeSet<_> = document
        .intents
        .iter()
        .map(|intent| intent.name.clone())
        .collect();
    for trigger in &document.application.triggers {
        if trigger.name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_TRIGGER_NAME_REQUIRED",
                "Triggers must declare a name.",
            ));
        }
        if !known_intents.contains(&trigger.target) {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_TRIGGER_INTENT",
                format!(
                    "Trigger '{}' targets unknown intent '{}'.",
                    trigger.name, trigger.target
                ),
            ));
        }
        diagnostics.extend(check_trigger_payload_fields(document, trigger));
        diagnostics.extend(check_trigger_bindings(document, trigger));
        diagnostics.extend(check_trigger_requirements(document, trigger));
    }
    diagnostics
}

fn check_trigger_payload_fields(
    document: &RifDocument,
    trigger: &TriggerDefinition,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_types = known_types(document);
    for (name, type_name) in &trigger.payload_fields {
        if name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_TRIGGER_PAYLOAD_NAME_REQUIRED",
                format!(
                    "Trigger '{}' declares a payload field with no name.",
                    trigger.name
                ),
            ));
        }
        if type_name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_TRIGGER_PAYLOAD_TYPE_REQUIRED",
                format!(
                    "Trigger '{}' payload field '{}' requires a type.",
                    trigger.name, name
                ),
            ));
            continue;
        }
        for referenced in referenced_types(type_name) {
            if !known_types.contains(&referenced) {
                diagnostics.push(Diagnostic::error(
                    "EIGL_UNKNOWN_TRIGGER_PAYLOAD_TYPE",
                    format!(
                        "Trigger '{}' payload field '{}' uses unknown type '{}'.",
                        trigger.name, name, referenced
                    ),
                ));
            }
        }
    }
    diagnostics
}

fn check_trigger_bindings(document: &RifDocument, trigger: &TriggerDefinition) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let Some(intent) = document
        .intents
        .iter()
        .find(|candidate| candidate.name == trigger.target)
    else {
        return diagnostics;
    };
    for (target, source) in &trigger.bindings {
        let target_type = expression_type(document, intent, target);
        if target_type.is_none() {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_TRIGGER_BINDING_TARGET",
                format!(
                    "Trigger '{}' binds unknown target '{}'.",
                    trigger.name, target
                ),
            ));
        }
        if source.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_TRIGGER_BINDING_SOURCE_REQUIRED",
                format!(
                    "Trigger '{}' binding '{}' requires a source expression.",
                    trigger.name, target
                ),
            ));
            continue;
        }
        let source_type = trigger_binding_source_type(document, intent, trigger, source);
        if source_type.is_none() && !trigger.payload_fields.is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_TRIGGER_BINDING_SOURCE",
                format!(
                    "Trigger '{}' binding '{}' refers to undeclared payload source '{}'.",
                    trigger.name, target, source
                ),
            ));
            continue;
        }
        if source_type.is_none() {
            let known = known_references(document);
            if !source.starts_with("event.")
                && !known.contains(source)
                && !known.contains(root_name(source))
            {
                diagnostics.push(Diagnostic::error(
                    "EIGL_UNKNOWN_TRIGGER_BINDING_SOURCE",
                    format!(
                        "Trigger '{}' binding '{}' refers to unknown source '{}'.",
                        trigger.name, target, source
                    ),
                ));
            }
        }
        if let (Some(source_type), Some(target_type)) = (source_type, target_type)
            && !types_compatible(&source_type, &target_type)
        {
            diagnostics.push(Diagnostic::error(
                "EIGL_TRIGGER_BINDING_TYPE_MISMATCH",
                format!(
                    "Trigger '{}' binding '{}' expects '{}' but source '{}' has type '{}'.",
                    trigger.name, target, target_type, source, source_type
                ),
            ));
        }
    }
    diagnostics
}

fn trigger_binding_source_type(
    document: &RifDocument,
    intent: &Intent,
    trigger: &TriggerDefinition,
    source: &str,
) -> Option<String> {
    let source = source.trim();
    trigger_binding_operand_type(document, intent, trigger, source).or_else(|| {
        arithmetic_expression_type(source, |operand| {
            trigger_binding_operand_type(document, intent, trigger, operand)
        })
    })
}

fn trigger_binding_operand_type(
    document: &RifDocument,
    intent: &Intent,
    trigger: &TriggerDefinition,
    source: &str,
) -> Option<String> {
    if let Some(type_name) = trigger.payload_fields.get(source) {
        return Some(type_name.clone());
    }
    if let Some(event_field) = source.strip_prefix("event.") {
        if let Some(type_name) = trigger.payload_fields.get(event_field) {
            return Some(type_name.clone());
        }
        if matches!(event_field, "name" | "kind" | "schedule" | "queue") {
            return Some("Text".to_string());
        }
        return None;
    }
    expression_type(document, intent, source)
}

fn check_trigger_requirements(
    document: &RifDocument,
    trigger: &TriggerDefinition,
) -> Vec<Diagnostic> {
    let known = known_references(document);
    let mut diagnostics = Vec::new();
    let Some(intent) = document
        .intents
        .iter()
        .find(|candidate| candidate.name == trigger.target)
    else {
        return diagnostics;
    };
    for requirement in &trigger.requires {
        for reference in predicate::references(requirement) {
            if !trigger_requirement_reference_is_known(
                document,
                intent,
                trigger,
                &known,
                requirement,
                &reference,
            ) {
                diagnostics.push(Diagnostic::error(
                    "EIGL_UNKNOWN_TRIGGER_REQUIREMENT_REFERENCE",
                    format!(
                        "Trigger '{}' requirement '{}' refers to unknown value '{}'.",
                        trigger.name, requirement, reference
                    ),
                ));
            }
        }
        diagnostics.extend(check_predicate_operand_types(
            requirement,
            &format!("Trigger '{}' requirement", trigger.name),
            None,
            |operand| trigger_binding_source_type(document, intent, trigger, operand),
        ));
    }
    diagnostics
}

fn trigger_requirement_reference_is_known(
    document: &RifDocument,
    intent: &Intent,
    trigger: &TriggerDefinition,
    known: &BTreeSet<String>,
    requirement: &str,
    reference: &str,
) -> bool {
    if reference.starts_with("event.") {
        return true;
    }
    if trigger_requirement_text_literal_reference(document, intent, trigger, requirement, reference)
    {
        return true;
    }
    predicate_reference_is_known(document, intent, known, reference)
}

fn trigger_requirement_text_literal_reference(
    document: &RifDocument,
    intent: &Intent,
    trigger: &TriggerDefinition,
    requirement: &str,
    reference: &str,
) -> bool {
    predicate::comparisons(requirement)
        .iter()
        .any(|comparison| {
            matches!(comparison.operator.as_str(), "is" | "is not")
                && comparison.right == reference
                && trigger_binding_source_type(document, intent, trigger, &comparison.left)
                    .as_deref()
                    == Some("Text")
                && trigger_binding_source_type(document, intent, trigger, &comparison.right)
                    .is_none()
        })
}

fn check_endpoint_bindings(
    document: &RifDocument,
    endpoint: &EndpointDefinition,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let allowed_request_sources = endpoint_allowed_request_sources(endpoint);
    let Some(intent) = document
        .intents
        .iter()
        .find(|candidate| candidate.name == endpoint.target)
    else {
        return diagnostics;
    };
    for (target, source) in &endpoint.bindings {
        let target_type = expression_type(document, intent, target);
        if target_type.is_none() {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_ENDPOINT_BINDING_TARGET",
                format!(
                    "Endpoint '{} {}' binds unknown target '{}'.",
                    endpoint.method, endpoint.path, target
                ),
            ));
        }
        if source.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_BINDING_SOURCE_REQUIRED",
                format!(
                    "Endpoint '{} {}' binding '{}' requires a source expression.",
                    endpoint.method, endpoint.path, target
                ),
            ));
            continue;
        }
        let source_type = endpoint_binding_source_type(
            document,
            intent,
            endpoint,
            &allowed_request_sources,
            source,
        );
        if source_type.is_none() && !endpoint.request_fields.is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_ENDPOINT_BINDING_SOURCE",
                format!(
                    "Endpoint '{} {}' binding '{}' refers to undeclared request source '{}'.",
                    endpoint.method, endpoint.path, target, source
                ),
            ));
            continue;
        }
        if let (Some(source_type), Some(target_type)) = (source_type, target_type)
            && !types_compatible(&source_type, &target_type)
        {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_BINDING_TYPE_MISMATCH",
                format!(
                    "Endpoint '{} {}' binding '{}' expects '{}' but source '{}' has type '{}'.",
                    endpoint.method, endpoint.path, target, target_type, source, source_type
                ),
            ));
        }
    }
    diagnostics
}

fn endpoint_allowed_request_sources(endpoint: &EndpointDefinition) -> BTreeSet<String> {
    let mut allowed = endpoint_path_params(&endpoint.path);
    allowed.extend(endpoint.request_fields.keys().cloned());
    allowed
}

fn endpoint_binding_source_type(
    document: &RifDocument,
    intent: &Intent,
    endpoint: &EndpointDefinition,
    allowed_request_sources: &BTreeSet<String>,
    source: &str,
) -> Option<String> {
    let source = source.trim();
    endpoint_binding_operand_type(document, intent, endpoint, allowed_request_sources, source)
        .or_else(|| {
            arithmetic_expression_type(source, |operand| {
                endpoint_binding_operand_type(
                    document,
                    intent,
                    endpoint,
                    allowed_request_sources,
                    operand,
                )
            })
        })
}

fn endpoint_binding_operand_type(
    document: &RifDocument,
    intent: &Intent,
    endpoint: &EndpointDefinition,
    allowed_request_sources: &BTreeSet<String>,
    source: &str,
) -> Option<String> {
    if let Some(type_name) = endpoint.request_fields.get(source) {
        return Some(type_name.clone());
    }
    if endpoint_path_params(&endpoint.path).contains(source) {
        return Some("Text".to_string());
    }
    if endpoint_framework_request_source(source) {
        return Some("Text".to_string());
    }
    expression_type(document, intent, source).or_else(|| {
        collection_expression_type_with_allowed_selectors(
            document,
            intent,
            source,
            allowed_request_sources,
        )
    })
}

fn endpoint_framework_request_source(source: &str) -> bool {
    source.starts_with("auth.")
        || source.starts_with("headers.")
        || source.starts_with("cookies.")
        || source.starts_with("query.")
}

fn check_endpoint_requirements(
    document: &RifDocument,
    endpoint: &EndpointDefinition,
) -> Vec<Diagnostic> {
    let known = known_references(document);
    let path_params = endpoint_path_params(&endpoint.path);
    let mut diagnostics = Vec::new();
    let Some(intent) = document
        .intents
        .iter()
        .find(|candidate| candidate.name == endpoint.target)
    else {
        return diagnostics;
    };
    for requirement in &endpoint.requires {
        let condition = endpoint_requirement_condition(requirement);
        for reference in predicate::references(condition) {
            if endpoint_requirement_reference_is_known(
                document,
                intent,
                endpoint,
                &known,
                &path_params,
                &reference,
            ) {
                continue;
            }
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_ENDPOINT_REQUIREMENT_REFERENCE",
                format!(
                    "Endpoint '{} {}' requirement '{}' refers to unknown value '{}'.",
                    endpoint.method, endpoint.path, requirement, reference
                ),
            ));
        }
        diagnostics.extend(check_predicate_operand_types(
            condition,
            &format!(
                "Endpoint '{} {}' requirement",
                endpoint.method, endpoint.path
            ),
            None,
            |operand| endpoint_expression_type(document, intent, endpoint, &path_params, operand),
        ));
    }
    diagnostics
}

fn endpoint_requirement_condition(requirement: &str) -> &str {
    requirement
        .rsplit_once(" else ")
        .map_or(requirement, |(condition, _)| condition.trim())
}

fn check_endpoint_responses(
    document: &RifDocument,
    endpoint: &EndpointDefinition,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let Some(intent) = document
        .intents
        .iter()
        .find(|candidate| candidate.name == endpoint.target)
    else {
        return diagnostics;
    };
    let known = known_references(document);
    if let Some(status) = endpoint.response_status.as_ref()
        && !valid_http_status(status)
    {
        diagnostics.push(Diagnostic::error(
            "EIGL_INVALID_ENDPOINT_RESPONSE_STATUS",
            format!(
                "Endpoint '{} {}' response status '{}' must start with an HTTP status code from 100 to 599.",
                endpoint.method, endpoint.path, status
            ),
        ));
    }
    if let Some(status) = endpoint.error_status.as_ref()
        && !valid_http_status(status)
    {
        diagnostics.push(Diagnostic::error(
            "EIGL_INVALID_ENDPOINT_ERROR_STATUS",
            format!(
                "Endpoint '{} {}' error status '{}' must start with an HTTP status code from 100 to 599.",
                endpoint.method, endpoint.path, status
            ),
        ));
    }
    for (name, error) in &endpoint.error_cases {
        if let Some(status) = error.status.as_ref()
            && !valid_http_status(status)
        {
            diagnostics.push(Diagnostic::error(
                "EIGL_INVALID_ENDPOINT_ERROR_STATUS",
                format!(
                    "Endpoint '{} {}' error '{}' status '{}' must start with an HTTP status code from 100 to 599.",
                    endpoint.method, endpoint.path, name, status
                ),
            ));
        }
    }
    diagnostics.extend(check_endpoint_response_fields(
        document,
        intent,
        endpoint,
        "response",
        &endpoint.response_fields,
        &endpoint.responses,
    ));
    diagnostics.extend(check_endpoint_response_fields(
        document,
        intent,
        endpoint,
        "error response",
        &endpoint.error_fields,
        &endpoint.error_responses,
    ));
    diagnostics.extend(check_endpoint_response_map(
        document,
        intent,
        &known,
        endpoint,
        "response",
        &endpoint.responses,
    ));
    diagnostics.extend(check_endpoint_response_map(
        document,
        intent,
        &known,
        endpoint,
        "error response",
        &endpoint.error_responses,
    ));
    for (name, error) in &endpoint.error_cases {
        diagnostics.extend(check_endpoint_response_fields(
            document,
            intent,
            endpoint,
            &format!("error '{name}' response"),
            &error.response_fields,
            &error.responses,
        ));
        diagnostics.extend(check_endpoint_response_map(
            document,
            intent,
            &known,
            endpoint,
            &format!("error '{name}' response"),
            &error.responses,
        ));
    }
    diagnostics
}

fn check_endpoint_response_fields(
    document: &RifDocument,
    intent: &Intent,
    endpoint: &EndpointDefinition,
    label: &str,
    fields: &BTreeMap<String, String>,
    responses: &BTreeMap<String, String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_types = known_types(document);
    let path_params = endpoint_path_params(&endpoint.path);
    for (name, type_name) in fields {
        if name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_RESPONSE_NAME_REQUIRED",
                format!(
                    "Endpoint '{} {}' declares a {} field with no name.",
                    endpoint.method, endpoint.path, label
                ),
            ));
        }
        if type_name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_RESPONSE_TYPE_REQUIRED",
                format!(
                    "Endpoint '{} {}' {} field '{}' requires a type.",
                    endpoint.method, endpoint.path, label, name
                ),
            ));
            continue;
        }
        for referenced in referenced_types(type_name) {
            if !known_types.contains(&referenced) {
                diagnostics.push(Diagnostic::error(
                    "EIGL_UNKNOWN_ENDPOINT_RESPONSE_TYPE",
                    format!(
                        "Endpoint '{} {}' {} field '{}' uses unknown type '{}'.",
                        endpoint.method, endpoint.path, label, name, referenced
                    ),
                ));
            }
        }
        let Some(source) = responses.get(name) else {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_RESPONSE_FIELD_UNMAPPED",
                format!(
                    "Endpoint '{} {}' declares {} field '{}' but does not map it.",
                    endpoint.method, endpoint.path, label, name
                ),
            ));
            continue;
        };
        if let Some(source_type) =
            endpoint_response_source_type(document, intent, endpoint, &path_params, source)
            && !types_compatible(&source_type, type_name)
        {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_RESPONSE_TYPE_MISMATCH",
                format!(
                    "Endpoint '{} {}' {} field '{}' expects '{}' but source '{}' has type '{}'.",
                    endpoint.method, endpoint.path, label, name, type_name, source, source_type
                ),
            ));
        }
    }
    diagnostics
}

fn check_endpoint_response_map(
    document: &RifDocument,
    intent: &Intent,
    known: &BTreeSet<String>,
    endpoint: &EndpointDefinition,
    label: &str,
    responses: &BTreeMap<String, String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let path_params = endpoint_path_params(&endpoint.path);
    for (name, source) in responses {
        if name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_RESPONSE_NAME_REQUIRED",
                format!(
                    "Endpoint '{} {}' has a {} with an empty name.",
                    endpoint.method, endpoint.path, label
                ),
            ));
        }
        if source.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_ENDPOINT_RESPONSE_SOURCE_REQUIRED",
                format!(
                    "Endpoint '{} {}' {} '{}' requires a source expression.",
                    endpoint.method, endpoint.path, label, name
                ),
            ));
        } else if endpoint_response_source_type(document, intent, endpoint, &path_params, source)
            .is_none()
            && !known.contains(source)
            && !known.contains(root_name(source))
        {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNKNOWN_ENDPOINT_RESPONSE_SOURCE",
                format!(
                    "Endpoint '{} {}' {} '{}' refers to unknown source '{}'.",
                    endpoint.method, endpoint.path, label, name, source
                ),
            ));
        }
    }
    diagnostics
}

fn endpoint_response_source_type(
    document: &RifDocument,
    intent: &Intent,
    endpoint: &EndpointDefinition,
    path_params: &BTreeSet<String>,
    source: &str,
) -> Option<String> {
    let source = source.trim();
    if source == "failure" {
        return Some("Text".to_string());
    }
    endpoint_expression_type(document, intent, endpoint, path_params, source).or_else(|| {
        arithmetic_expression_type(source, |operand| {
            if operand == "failure" {
                Some("Text".to_string())
            } else {
                endpoint_expression_type(document, intent, endpoint, path_params, operand)
            }
        })
    })
}

fn valid_http_status(status: &str) -> bool {
    let Some(code) = status.split_whitespace().next() else {
        return false;
    };
    code.len() == 3
        && code.chars().all(|ch| ch.is_ascii_digit())
        && code
            .parse::<u16>()
            .is_ok_and(|value| (100..=599).contains(&value))
}

fn check_call_targets(document: &RifDocument, steps: &[Step]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for step in steps {
        for call in step_calls(step) {
            if call.target.is_empty() {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_CALL_TARGET_REQUIRED",
                        format!(
                            "Step '{}' has a call without a target operation name.",
                            step.title
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }
            if has_application_declarations(document)
                && !document.application.operations.contains_key(&call.target)
            {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_UNKNOWN_OPERATION",
                        format!(
                            "Step '{}' calls unknown operation '{}'.",
                            step.title, call.target
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }
        }
    }
    diagnostics
}

fn check_invoke_targets(document: &RifDocument, steps: &[Step]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_intents: BTreeSet<_> = document
        .intents
        .iter()
        .map(|intent| intent.name.clone())
        .collect();
    for step in steps {
        for invoke in step_invokes(step) {
            if invoke.target.is_empty() {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_INVOKE_TARGET_REQUIRED",
                        format!(
                            "Step '{}' has an intent invocation without a target.",
                            step.title
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
                continue;
            }
            if !known_intents.contains(&invoke.target) {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_UNKNOWN_INTENT",
                        format!(
                            "Step '{}' invokes unknown intent '{}'.",
                            step.title, invoke.target
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }
        }
    }
    diagnostics
}

fn check_invocation_bindings(document: &RifDocument, intent: &Intent) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for step in &intent.steps {
        for invocation in step_invocations(step) {
            let Some(target_intent) = document
                .intents
                .iter()
                .find(|candidate| candidate.name == invocation.target)
            else {
                continue;
            };
            let mut expected_types: BTreeMap<&str, &str> = BTreeMap::new();
            for (name, thing) in target_intent
                .subjects
                .iter()
                .chain(target_intent.inputs.iter())
            {
                expected_types.insert(name.as_str(), thing.type_name.as_str());
            }
            for (name, expected_type) in &expected_types {
                if invocation.bindings.contains_key(*name) {
                    continue;
                }
                match implicit_invocation_binding_type(intent, name) {
                    Some(actual_type) if types_compatible(actual_type, expected_type) => {}
                    Some(actual_type) => diagnostics.push(
                        Diagnostic::error(
                            "EIGL_INVOKE_BINDING_TYPE",
                            format!(
                                "Step '{}' invokes '{}' implicit binding '{}' with type '{}', but expected '{}'.",
                                step.title, invocation.target, name, actual_type, expected_type
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    ),
                    None => diagnostics.push(
                        Diagnostic::error(
                            "EIGL_MISSING_INVOKE_BINDING",
                            format!(
                                "Step '{}' invokes '{}' without required binding '{}'.",
                                step.title, invocation.target, name
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    ),
                }
            }
            for (name, value) in &invocation.bindings {
                let Some(expected_type) = expected_types.get(name.as_str()) else {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_UNKNOWN_INVOKE_BINDING",
                            format!(
                                "Step '{}' invokes '{}' with unknown binding '{}'.",
                                step.title, invocation.target, name
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                    continue;
                };
                let Some(actual_type) = value_expression_type(document, intent, value) else {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_UNKNOWN_INVOKE_BINDING_REFERENCE",
                            format!(
                                "Step '{}' invokes '{}' with unknown binding value '{}'.",
                                step.title, invocation.target, value
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                    continue;
                };
                if !types_compatible(&actual_type, expected_type) {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_INVOKE_BINDING_TYPE",
                            format!(
                                "Step '{}' invokes '{}' binding '{}' with type '{}', but expected '{}'.",
                                step.title, invocation.target, name, actual_type, expected_type
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                }
            }
        }
    }
    diagnostics
}

fn implicit_invocation_binding_type<'a>(intent: &'a Intent, name: &str) -> Option<&'a str> {
    intent
        .subjects
        .get(name)
        .or_else(|| intent.inputs.get(name))
        .map(|thing| thing.type_name.as_str())
}

fn check_operation_call_signatures(document: &RifDocument, intent: &Intent) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for step in &intent.steps {
        for call in step_calls(step) {
            let Some(operation) = document.application.operations.get(&call.target) else {
                continue;
            };
            if call.args.len() != operation.input_order.len() {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_OPERATION_ARGUMENT_COUNT",
                        format!(
                            "Step '{}' calls '{}' with {} argument(s), but the operation expects {}.",
                            step.title,
                            call.target,
                            call.args.len(),
                            operation.input_order.len()
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
                continue;
            }

            if (operation_requires_stored_outputs(operation) || !step.outputs.is_empty())
                && step.outputs.len() != operation.outputs.len()
            {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_OPERATION_OUTPUT_COUNT",
                        format!(
                            "Step '{}' stores {} output value(s), but operation '{}' returns {}.",
                            step.title,
                            step.outputs.len(),
                            call.target,
                            operation.outputs.len()
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }

            for (index, arg) in call.args.iter().enumerate() {
                let input_name = &operation.input_order[index];
                let Some(expected_type) = operation.inputs.get(input_name) else {
                    continue;
                };
                let Some(actual_type) = value_expression_type(document, intent, arg) else {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_UNKNOWN_CALL_ARGUMENT_REFERENCE",
                            format!(
                                "Step '{}' calls '{}' with unknown argument reference '{}'.",
                                step.title, call.target, arg
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                    continue;
                };
                if !types_compatible(&actual_type, expected_type) {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_OPERATION_ARGUMENT_TYPE",
                            format!(
                                "Step '{}' calls '{}' argument '{}' for parameter '{}' with type '{}', but expected '{}'.",
                                step.title, call.target, arg, input_name, actual_type, expected_type
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                }
            }

            for output in step.outputs.values() {
                let Some(expected_output) = operation_output_contract(operation, &output.name)
                else {
                    if !operation.outputs.is_empty() {
                        diagnostics.push(
                            Diagnostic::error(
                                "EIGL_OPERATION_OUTPUT_NAME",
                                format!(
                                    "Step '{}' stores output '{}', but operation '{}' does not return that named output.",
                                    step.title, output.name, call.target
                                ),
                            )
                            .at(format!("step {}", step.number)),
                        );
                    }
                    continue;
                };
                if !types_compatible(&output.type_name, &expected_output.type_name) {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_OPERATION_OUTPUT_TYPE",
                            format!(
                                "Step '{}' stores output '{}' with type '{}', but operation '{}' returns '{}'.",
                                step.title,
                                output.name,
                                output.type_name,
                                call.target,
                                expected_output.type_name
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                }
            }
        }
    }
    diagnostics
}

fn operation_output_contract<'a>(
    operation: &'a crate::rif_model::OperationDefinition,
    name: &str,
) -> Option<&'a OutputValue> {
    let output = operation.outputs.first()?;
    if operation.outputs.len() == 1 && output.name == "result" {
        return Some(output);
    }
    operation.outputs.iter().find(|output| output.name == name)
}

fn operation_requires_stored_outputs(operation: &crate::rif_model::OperationDefinition) -> bool {
    operation
        .outputs
        .iter()
        .any(|output| output.name != "result")
}

fn has_application_declarations(document: &RifDocument) -> bool {
    document.application.name.is_some()
        || !document.application.enums.is_empty()
        || !document.application.things.is_empty()
        || !document.application.operations.is_empty()
}

fn check_output_types(document: &RifDocument, steps: &[Step]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_types = known_types(document);
    for step in steps {
        for output in step.outputs.values() {
            if output.type_name.is_empty() {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_OUTPUT_TYPE_REQUIRED",
                        format!(
                            "Step '{}' output '{}' must declare a type.",
                            step.title, output.name
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            } else if !document.application.things.is_empty() {
                for referenced in referenced_types(&output.type_name) {
                    if !known_types.contains(&referenced) {
                        diagnostics.push(
                            Diagnostic::error(
                                "EIGL_UNKNOWN_TYPE",
                                format!(
                                    "Step '{}' output '{}' uses unknown type '{}'.",
                                    step.title, output.name, referenced
                                ),
                            )
                            .at(format!("step {}", step.number)),
                        );
                    }
                }
            }
        }
    }
    diagnostics
}

fn check_failure_completeness(
    document: &RifDocument,
    steps: &[Step],
    handlers: &[FailureCase],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for step in steps {
        for failure in effective_failures(document, step) {
            if step.ignored_failures.contains(failure) {
                continue;
            }
            if !failure_is_handled(failure, step, handlers) {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_UNHANDLED_FAILURE",
                        format!(
                            "Step '{}' declares failure '{}', but it is not handled, returned with stop with, or explicitly ignored.",
                            step.title, failure
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }
        }
    }
    diagnostics
}

fn effective_failures<'a>(document: &'a RifDocument, step: &'a Step) -> BTreeSet<&'a String> {
    let mut failures: BTreeSet<&String> = step.may_fail.iter().collect();
    for call in step_calls(step) {
        if let Some(operation) = document.application.operations.get(&call.target) {
            failures.extend(operation.may_fail.iter());
        }
    }
    failures
}

fn check_unordered_change_conflicts(steps: &[Step], schedule: &str) -> Vec<Diagnostic> {
    if schedule != "unordered" {
        return Vec::new();
    }
    let mut first_writer: BTreeMap<String, &Step> = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for step in steps {
        let mut changes: BTreeSet<String> = step.changes.iter().cloned().collect();
        for statement in step_assignments(step) {
            if let Some((left, _)) = split_top_level_once(statement, '=') {
                changes.insert(left.trim().to_string());
            }
        }
        for statement in step_appends(step) {
            if let Some((left, _)) = statement.split_once("+=") {
                changes.insert(left.trim().to_string());
            }
        }
        for statement in step_deletes(step) {
            changes.insert(statement.trim().to_string());
        }
        for target in changes {
            if let Some(previous) = first_writer.get(&target) {
                diagnostics.push(Diagnostic::error(
                    "EIGL_PERMISSION_CONFLICT",
                    format!(
                        "Unordered steps '{}' and '{}' both change '{}'.",
                        previous.title, step.title, target
                    ),
                ));
            } else {
                first_writer.insert(target, step);
            }
        }
    }
    diagnostics
}

fn check_set_assignments(document: &RifDocument, intent: &Intent) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for step in &intent.steps {
        for statement in step_assignments(step) {
            let Some((target, value)) = split_top_level_once(statement, '=') else {
                continue;
            };
            let target = target.trim();
            let value = value.trim();
            diagnostics.extend(check_collection_selector_types(
                document,
                intent,
                value,
                &format!("Step '{}' assignment", step.title),
                Some(format!("step {}", step.number)),
            ));
            let Some(target_type) = expression_type(document, intent, target) else {
                if has_application_declarations(document) {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_UNKNOWN_ASSIGNMENT_TARGET",
                            format!(
                                "Step '{}' assigns to unknown target '{}'.",
                                step.title, target
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                }
                continue;
            };

            if let Some(states) = state_type_values(&target_type) {
                if states.iter().any(|state| state == value) {
                    continue;
                }
                if value_expression_type(document, intent, value)
                    .is_some_and(|value_type| types_compatible(&value_type, &target_type))
                {
                    continue;
                }
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_ASSIGNMENT_STATE_VALUE",
                        format!(
                            "Step '{}' assigns '{}' to '{}', but '{}' expects one of {}.",
                            step.title,
                            value,
                            target,
                            target,
                            states.join(", ")
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
                continue;
            }

            if let Some(values) = enum_type_values(document, &target_type) {
                if values.iter().any(|enum_value| enum_value == value) {
                    continue;
                }
                if value_expression_type(document, intent, value)
                    .is_some_and(|value_type| types_compatible(&value_type, &target_type))
                {
                    continue;
                }
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_ASSIGNMENT_ENUM_VALUE",
                        format!(
                            "Step '{}' assigns '{}' to '{}', but '{}' expects one of {}.",
                            step.title,
                            value,
                            target,
                            target,
                            values.join(", ")
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
                continue;
            }

            let Some(value_type) = value_expression_type(document, intent, value) else {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_UNKNOWN_ASSIGNMENT_REFERENCE",
                        format!(
                            "Step '{}' assigns unknown value '{}' to '{}'.",
                            step.title, value, target
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
                continue;
            };

            if !types_compatible(&value_type, &target_type) {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_ASSIGNMENT_TYPE",
                        format!(
                            "Step '{}' assigns '{}' with type '{}' to '{}', but expected '{}'.",
                            step.title, value, value_type, target, target_type
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }
        }
    }
    diagnostics
}

fn value_expression_type(document: &RifDocument, intent: &Intent, value: &str) -> Option<String> {
    expression_type(document, intent, value).or_else(|| {
        arithmetic_expression_type(value, |operand| expression_type(document, intent, operand))
    })
}

fn check_append_assignments(document: &RifDocument, intent: &Intent) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for step in &intent.steps {
        for statement in step_appends(step) {
            let Some((target, value)) = statement.split_once("+=") else {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_APPEND_SYNTAX",
                        format!(
                            "Step '{}' append statement '{}' must use 'target += value'.",
                            step.title, statement
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
                continue;
            };
            let target = target.trim();
            let value = value.trim();
            diagnostics.extend(check_collection_selector_types(
                document,
                intent,
                value,
                &format!("Step '{}' append", step.title),
                Some(format!("step {}", step.number)),
            ));
            let Some(target_type) = expression_type(document, intent, target) else {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_UNKNOWN_APPEND_TARGET",
                        format!(
                            "Step '{}' appends to unknown target '{}'.",
                            step.title, target
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
                continue;
            };
            let Some(element_type) = generic_inner(&target_type, "List") else {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_APPEND_TARGET_TYPE",
                        format!(
                            "Step '{}' appends to '{}', but '{}' has type '{}' instead of a List type.",
                            step.title, target, target, target_type
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
                continue;
            };
            let Some(value_type) = append_value_type(document, intent, element_type, value) else {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_UNKNOWN_APPEND_VALUE",
                        format!(
                            "Step '{}' appends unknown value '{}' to '{}'.",
                            step.title, value, target
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
                continue;
            };
            if !types_compatible(&value_type, element_type) {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_APPEND_VALUE_TYPE",
                        format!(
                            "Step '{}' appends '{}' with type '{}' to '{}', but list elements must be '{}'.",
                            step.title, value, value_type, target, element_type
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }
        }
    }
    diagnostics
}

fn append_value_type(
    document: &RifDocument,
    intent: &Intent,
    element_type: &str,
    value: &str,
) -> Option<String> {
    if state_type_values(element_type)
        .is_some_and(|states| states.iter().any(|state| state == value))
    {
        return Some(element_type.to_string());
    }
    if enum_type_values(document, element_type)
        .is_some_and(|values| values.iter().any(|enum_value| enum_value == value))
    {
        return Some(element_type.to_string());
    }
    value_expression_type(document, intent, value)
}

fn check_requirements(document: &RifDocument, intent: &Intent) -> Vec<Diagnostic> {
    let known = known_references(document);
    let mut diagnostics = Vec::new();
    for requirement in &intent.requires {
        for reference in predicate::references(&requirement.text) {
            if !predicate_reference_is_known(document, intent, &known, &reference) {
                diagnostics.push(Diagnostic::error(
                    "EIGL_UNKNOWN_REQUIREMENT_REFERENCE",
                    format!(
                        "Requirement '{}' refers to unknown value '{}'.",
                        requirement.text, reference
                    ),
                ));
            }
            diagnostics.extend(check_collection_selector_types(
                document,
                intent,
                &reference,
                "Requirement",
                None,
            ));
        }
        diagnostics.extend(check_predicate_operand_types(
            &requirement.text,
            "Requirement",
            None,
            |operand| expression_type(document, intent, operand),
        ));
    }
    diagnostics
}

fn check_guards(document: &RifDocument, intent: &crate::rif_model::Intent) -> Vec<Diagnostic> {
    let known = known_references(document);
    let mut diagnostics = Vec::new();
    for step in &intent.steps {
        let Some(guard) = &step.guard else {
            continue;
        };
        for reference in predicate::references(guard) {
            if !predicate_reference_is_known(document, intent, &known, &reference) {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_UNKNOWN_GUARD_REFERENCE",
                        format!(
                            "Step '{}' guard '{}' refers to unknown value '{}'.",
                            step.title, guard, reference
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }
            diagnostics.extend(check_collection_selector_types(
                document,
                intent,
                &reference,
                &format!("Step '{}' guard", step.title),
                Some(format!("step {}", step.number)),
            ));
        }
        diagnostics.extend(check_predicate_operand_types(
            guard,
            &format!("Step '{}' guard", step.title),
            Some(format!("step {}", step.number)),
            |operand| expression_type(document, intent, operand),
        ));
    }
    diagnostics
}

fn check_repeats(document: &RifDocument, intent: &crate::rif_model::Intent) -> Vec<Diagnostic> {
    let known = known_references(document);
    let mut diagnostics = Vec::new();
    for step in &intent.steps {
        if step.repeat_while.is_some() && step.repeat_until.is_some() {
            diagnostics.push(
                Diagnostic::error(
                    "EIGL_REPEAT_CONFLICT",
                    format!(
                        "Step '{}' cannot declare both 'repeat while' and 'repeat until'.",
                        step.title
                    ),
                )
                .at(format!("step {}", step.number)),
            );
        }
        for condition in [&step.repeat_while, &step.repeat_until]
            .into_iter()
            .flatten()
        {
            for reference in predicate::references(condition) {
                if !predicate_reference_is_known(document, intent, &known, &reference) {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_UNKNOWN_REPEAT_REFERENCE",
                            format!(
                                "Step '{}' repeat condition '{}' refers to unknown value '{}'.",
                                step.title, condition, reference
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                }
                diagnostics.extend(check_collection_selector_types(
                    document,
                    intent,
                    &reference,
                    &format!("Step '{}' repeat condition", step.title),
                    Some(format!("step {}", step.number)),
                ));
            }
            diagnostics.extend(check_predicate_operand_types(
                condition,
                &format!("Step '{}' repeat condition", step.title),
                Some(format!("step {}", step.number)),
                |operand| expression_type(document, intent, operand),
            ));
        }
    }
    diagnostics
}

fn check_predicate_operand_types<F>(
    predicate_text: &str,
    context: &str,
    location: Option<String>,
    mut resolve_type: F,
) -> Vec<Diagnostic>
where
    F: FnMut(&str) -> Option<String>,
{
    let mut diagnostics = Vec::new();
    for comparison in predicate::comparisons(predicate_text) {
        let left_type = predicate_operand_type(
            &comparison.left,
            None,
            &comparison.operator,
            &mut resolve_type,
        );
        let right_type = predicate_operand_type(
            &comparison.right,
            left_type.as_deref(),
            &comparison.operator,
            &mut resolve_type,
        );
        let (Some(left_type), Some(right_type)) = (left_type, right_type) else {
            continue;
        };
        if predicate_operator_types_compatible(&comparison.operator, &left_type, &right_type) {
            continue;
        }

        let diagnostic = Diagnostic::error(
            "EIGL_PREDICATE_OPERAND_TYPE",
            format!(
                "{} '{}' compares '{}' ({}) {} '{}' ({}), which is not a valid predicate operand combination.",
                context,
                predicate_text.trim(),
                comparison.left,
                left_type,
                comparison.operator,
                comparison.right,
                right_type
            ),
        );
        diagnostics.push(match &location {
            Some(location) => diagnostic.at(location.clone()),
            None => diagnostic,
        });
    }
    diagnostics
}

fn predicate_operand_type<F>(
    operand: &str,
    expected_type: Option<&str>,
    operator: &str,
    resolve_type: &mut F,
) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let operand = operand.trim();
    if matches!(operator, "is" | "is not")
        && let Some(expected_type) = expected_type
    {
        if expected_type == "Text"
            && !predicate_operand_looks_like_reference(operand)
            && !operand.starts_with('"')
        {
            return Some("Text".to_string());
        }
        if state_type_values(expected_type)
            .is_some_and(|states| states.iter().any(|state| state == operand))
        {
            return Some(expected_type.to_string());
        }
    }
    if operator == "contains"
        && let Some(expected_type) = expected_type
        && let Some(contained_type) = contained_type_for_contains(expected_type)
    {
        if contained_type == "Text"
            && !predicate_operand_looks_like_reference(operand)
            && !operand.starts_with('"')
        {
            return Some("Text".to_string());
        }
        if state_type_values(&contained_type)
            .is_some_and(|states| states.iter().any(|state| state == operand))
        {
            return Some(contained_type);
        }
    }
    if let Some(operand_type) = resolve_type(operand) {
        return Some(operand_type);
    }
    arithmetic_expression_type(operand, |inner| resolve_type(inner))
}

fn predicate_operator_types_compatible(operator: &str, left_type: &str, right_type: &str) -> bool {
    match operator {
        ">" | "<" | ">=" | "<=" => predicate_ordering_types_compatible(left_type, right_type),
        "==" | "!=" | "is" | "is not" => {
            types_compatible(left_type, right_type) || types_compatible(right_type, left_type)
        }
        "contains" => predicate_contains_types_compatible(left_type, right_type),
        _ => false,
    }
}

fn predicate_contains_types_compatible(container_type: &str, member_type: &str) -> bool {
    if container_type == "Text" {
        return types_compatible(member_type, "Text");
    }
    let Some(contained_type) = contained_type_for_contains(container_type) else {
        return false;
    };
    types_compatible(member_type, &contained_type) || types_compatible(&contained_type, member_type)
}

fn contained_type_for_contains(container_type: &str) -> Option<String> {
    if container_type.trim() == "Text" {
        return Some("Text".to_string());
    }
    if let Some(element_type) = generic_inner(container_type, "List") {
        return Some(element_type.to_string());
    }
    let map_types = generic_args(container_type, "Map")?;
    (map_types.len() == 2).then(|| map_types[0].to_string())
}

fn predicate_ordering_types_compatible(left_type: &str, right_type: &str) -> bool {
    let numeric = matches!(left_type, "Int" | "Decimal") && matches!(right_type, "Int" | "Decimal");
    numeric
        || matches!(
            (left_type, right_type),
            ("Money", "Money") | ("Time", "Time") | ("Duration", "Duration")
        )
}

fn check_compute_expressions(
    document: &RifDocument,
    intent: &crate::rif_model::Intent,
) -> Vec<Diagnostic> {
    let known = known_references(document);
    let mut diagnostics = Vec::new();
    for step in &intent.steps {
        for statement in step_computes(step) {
            let Some((field, expression)) = split_top_level_once(statement, '=') else {
                continue;
            };
            let field = field.trim();
            if !known.contains(field) {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_UNKNOWN_EXPRESSION_REFERENCE",
                        format!(
                            "Step '{}' compute target '{}' is not a known field or value.",
                            step.title, field
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }
            for reference in expression::references(expression) {
                if !known.contains(&reference) && !known.contains(root_name(&reference)) {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_UNKNOWN_EXPRESSION_REFERENCE",
                            format!(
                                "Step '{}' compute expression '{}' refers to unknown value '{}'.",
                                step.title,
                                expression.trim(),
                                reference
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                }
            }
            let target_type = expression_type(document, intent, field);
            let expression_type =
                compute_expression_type(document, intent, step, expression, &mut diagnostics);
            if let (Some(actual_type), Some(expected_type)) = (expression_type, target_type)
                && !types_compatible(&actual_type, &expected_type)
            {
                diagnostics.push(
                    Diagnostic::error(
                        "EIGL_COMPUTE_TYPE",
                        format!(
                            "Step '{}' computes '{}' with type '{}', but '{}' expects '{}'.",
                            step.title,
                            expression.trim(),
                            actual_type,
                            field,
                            expected_type
                        ),
                    )
                    .at(format!("step {}", step.number)),
                );
            }
        }
    }
    diagnostics
}

fn check_returns(document: &RifDocument, intent: &Intent) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for return_value in &intent.returns {
        if return_value.name.trim().is_empty() {
            diagnostics.push(
                Diagnostic::error(
                    "EIGL_RETURN_NAME_REQUIRED",
                    "Intent return values must declare a name.",
                )
                .at(format!("intent {}", intent.name)),
            );
        }
        if value_expression_type(document, intent, &return_value.source).is_none() {
            diagnostics.push(
                Diagnostic::error(
                    "EIGL_UNKNOWN_RETURN_REFERENCE",
                    format!(
                        "Intent '{}' return '{}' refers to unknown source '{}'.",
                        intent.name, return_value.name, return_value.source
                    ),
                )
                .at(format!("intent {}", intent.name)),
            );
        }
    }
    diagnostics
}

fn compute_expression_type(
    document: &RifDocument,
    intent: &Intent,
    step: &Step,
    expression: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    let tree = expression::arithmetic_tree(expression)?;
    compute_node_type(document, intent, step, expression, &tree, diagnostics)
}

fn compute_node_type(
    document: &RifDocument,
    intent: &Intent,
    step: &Step,
    expression: &str,
    node: &expression::ArithmeticNode,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    match node {
        expression::ArithmeticNode::Operand(operand) => expression_type(document, intent, operand),
        expression::ArithmeticNode::Binary {
            operator,
            left,
            right,
        } => {
            let left_type =
                compute_node_type(document, intent, step, expression, left, diagnostics);
            let right_type =
                compute_node_type(document, intent, step, expression, right, diagnostics);
            let (Some(left_type), Some(right_type)) = (left_type, right_type) else {
                return None;
            };
            arithmetic_result_type(&left_type, *operator, &right_type).or_else(|| {
                diagnostics.push(compute_operator_type_diagnostic(
                    step,
                    expression,
                    &left_type,
                    *operator,
                    &right_type,
                ));
                None
            })
        }
    }
}

fn arithmetic_result_type(left_type: &str, operator: char, right_type: &str) -> Option<String> {
    match (left_type, operator, right_type) {
        ("Int", _, "Int") => Some("Int".to_string()),
        ("Int" | "Decimal", _, "Int" | "Decimal") => Some("Decimal".to_string()),
        ("Money", '+' | '-', "Money") => Some("Money".to_string()),
        ("Money", '*' | '/', "Int" | "Decimal") => Some("Money".to_string()),
        ("Int" | "Decimal", '*', "Money") => Some("Money".to_string()),
        ("Text", '+', "Text") => Some("Text".to_string()),
        _ => None,
    }
}

fn arithmetic_expression_type<F>(expression: &str, mut resolve_type: F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let tree = expression::arithmetic_tree(expression)?;
    arithmetic_node_type(&tree, &mut resolve_type)
}

fn arithmetic_node_type<F>(
    node: &expression::ArithmeticNode,
    resolve_type: &mut F,
) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    match node {
        expression::ArithmeticNode::Operand(operand) => resolve_type(operand),
        expression::ArithmeticNode::Binary {
            operator,
            left,
            right,
        } => {
            let left_type = arithmetic_node_type(left, resolve_type)?;
            let right_type = arithmetic_node_type(right, resolve_type)?;
            arithmetic_result_type(&left_type, *operator, &right_type)
        }
    }
}

fn compute_operator_type_diagnostic(
    step: &Step,
    expression: &str,
    left_type: &str,
    operator: char,
    right_type: &str,
) -> Diagnostic {
    Diagnostic::error(
        "EIGL_COMPUTE_OPERAND_TYPE",
        format!(
            "Step '{}' computes '{}' with operator '{}' between '{}' and '{}', which is not a valid arithmetic combination.",
            step.title,
            expression.trim(),
            operator,
            left_type,
            right_type
        ),
    )
    .at(format!("step {}", step.number))
}

fn check_secret_flows(document: &RifDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for intent in &document.intents {
        let secret_names: BTreeSet<_> = intent
            .subjects
            .iter()
            .chain(intent.inputs.iter())
            .filter(|(_, thing)| thing.is_secret)
            .map(|(name, _)| name.clone())
            .collect();
        for step in &intent.steps {
            for call in step_calls(step) {
                let secret_args: Vec<_> = call
                    .args
                    .iter()
                    .filter(|arg| {
                        secret_names.contains(root_name(arg))
                            || value_expression_type(document, intent, arg)
                                .is_some_and(|type_name| type_contains_secret(document, &type_name))
                    })
                    .cloned()
                    .collect();
                if secret_args.is_empty() {
                    continue;
                }
                let target = call.target.to_ascii_lowercase();
                let external = step.external_calls.join(" ").to_ascii_lowercase();
                let allowed_transform = target.starts_with("hash.")
                    || target.starts_with("crypto.")
                    || target.starts_with("passwordhash.");
                if target.contains("log")
                    || external.contains("log")
                    || (!step.external_calls.is_empty() && !allowed_transform)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_SECRET_FLOW",
                            format!(
                                "Step '{}' sends secret value(s) {} to '{}', which is not an authorized secret sink.",
                                step.title,
                                secret_args.join(", "),
                                call.target
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    )
                }
            }
        }
        for return_value in &intent.returns {
            let returns_secret = secret_names.contains(root_name(&return_value.source))
                || value_expression_type(document, intent, &return_value.source)
                    .is_some_and(|type_name| type_contains_secret(document, &type_name));
            if !returns_secret {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    "EIGL_SECRET_FLOW",
                    format!(
                        "Intent '{}' returns secret value '{}' via '{}'.",
                        intent.name, return_value.source, return_value.name
                    ),
                )
                .at(format!("intent {}", intent.name)),
            );
        }
    }
    diagnostics.extend(check_endpoint_secret_response_flows(document));
    diagnostics
}

fn check_endpoint_secret_response_flows(document: &RifDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for endpoint in &document.application.endpoints {
        let Some(intent) = document
            .intents
            .iter()
            .find(|candidate| candidate.name == endpoint.target)
        else {
            continue;
        };
        let path_params = endpoint_path_params(&endpoint.path);
        diagnostics.extend(check_endpoint_secret_response_map(
            document,
            intent,
            endpoint,
            &path_params,
            "response",
            &endpoint.responses,
        ));
        diagnostics.extend(check_endpoint_secret_response_map(
            document,
            intent,
            endpoint,
            &path_params,
            "error response",
            &endpoint.error_responses,
        ));
        for (name, error) in &endpoint.error_cases {
            let label = format!("error '{name}' response");
            diagnostics.extend(check_endpoint_secret_response_map(
                document,
                intent,
                endpoint,
                &path_params,
                &label,
                &error.responses,
            ));
        }
    }
    diagnostics
}

fn check_endpoint_secret_response_map(
    document: &RifDocument,
    intent: &Intent,
    endpoint: &EndpointDefinition,
    path_params: &BTreeSet<String>,
    label: &str,
    responses: &BTreeMap<String, String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (name, source) in responses {
        let exposes_secret =
            endpoint_response_source_type(document, intent, endpoint, path_params, source)
                .is_some_and(|type_name| type_contains_secret(document, &type_name));
        if exposes_secret {
            diagnostics.push(
                Diagnostic::error(
                    "EIGL_SECRET_FLOW",
                    format!(
                        "Endpoint '{} {}' {} '{}' exposes secret value '{}'.",
                        endpoint.method, endpoint.path, label, name, source
                    ),
                )
                .at(format!("endpoint {} {}", endpoint.method, endpoint.path)),
            );
        }
    }
    diagnostics
}

fn step_invocations(step: &Step) -> Vec<&InvocationTarget> {
    let mut invocations = Vec::new();
    if let Some(invoke) = &step.invoke {
        invocations.push(invoke);
    }
    invocations.extend(step.parallel_invokes.iter());
    if let Some(invoke) = &step.otherwise_invoke {
        invocations.push(invoke);
    }
    invocations.extend(step.otherwise_parallel_invokes.iter());
    invocations
}

fn check_guarantees(document: &RifDocument) -> Vec<Diagnostic> {
    let known = known_references(document);
    let mut diagnostics = Vec::new();
    for intent in &document.intents {
        for guarantee in &intent.guarantees {
            for statement in &guarantee.statements {
                for reference in predicate::references(statement) {
                    if !predicate_reference_is_known(document, intent, &known, &reference) {
                        diagnostics.push(Diagnostic::error(
                            "EIGL_UNKNOWN_GUARANTEE_REFERENCE",
                            format!(
                                "Guarantee '{}' refers to unknown subject, field, state, or output '{}'.",
                                statement, reference
                            ),
                        ));
                    }
                    diagnostics.extend(check_collection_selector_types(
                        document,
                        intent,
                        &reference,
                        "Guarantee",
                        None,
                    ));
                }
                diagnostics.extend(check_predicate_operand_types(
                    statement,
                    "Guarantee",
                    None,
                    |operand| expression_type(document, intent, operand),
                ));
            }
        }
    }
    diagnostics
}

fn check_unresolved_questions(document: &RifDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for intent in &document.intents {
        if !intent.unresolved_questions.is_empty() {
            diagnostics.push(Diagnostic::error(
                "EIGL_UNRESOLVED_QUESTIONS_PRESENT",
                format!(
                    "Intent '{}' still contains {} unresolved question(s).",
                    intent.name,
                    intent.unresolved_questions.len()
                ),
            ));
        }
    }
    diagnostics
}

fn predicate_reference_is_known(
    document: &RifDocument,
    intent: &Intent,
    known: &BTreeSet<String>,
    reference: &str,
) -> bool {
    if has_application_declarations(document) {
        expression_type(document, intent, reference).is_some()
            || known.contains(reference)
            || known.contains(root_name(reference))
    } else {
        known.contains(reference) || known.contains(root_name(reference))
    }
}

fn endpoint_requirement_reference_is_known(
    document: &RifDocument,
    intent: &Intent,
    endpoint: &EndpointDefinition,
    known: &BTreeSet<String>,
    path_params: &BTreeSet<String>,
    reference: &str,
) -> bool {
    if endpoint_request_reference_is_known(endpoint, path_params, reference) {
        return true;
    }

    let root = collection_aware_root_name(reference);
    if intent.subjects.contains_key(root)
        || intent.inputs.contains_key(root)
        || document.application.collections.contains_key(root)
    {
        return endpoint_expression_type(document, intent, endpoint, path_params, reference)
            .is_some();
    }

    if reference.contains('.') {
        return false;
    }

    predicate_reference_is_known(document, intent, known, reference)
}

fn endpoint_request_reference_is_known(
    endpoint: &EndpointDefinition,
    path_params: &BTreeSet<String>,
    reference: &str,
) -> bool {
    let root = root_name(reference);
    endpoint.request_fields.contains_key(reference)
        || matches!(root, "auth" | "headers" | "cookies" | "query")
        || path_params.contains(root)
}

fn collection_aware_root_name(reference: &str) -> &str {
    let root = root_name(reference);
    root.split_once('[').map_or(root, |(name, _)| name)
}

fn endpoint_expression_type(
    document: &RifDocument,
    intent: &Intent,
    endpoint: &EndpointDefinition,
    path_params: &BTreeSet<String>,
    expression: &str,
) -> Option<String> {
    if let Some(type_name) = endpoint.request_fields.get(expression) {
        return Some(type_name.clone());
    }
    if path_params.contains(expression) {
        return Some("Text".to_string());
    }
    if endpoint_framework_request_source(expression) {
        return Some("Text".to_string());
    }
    if let Some(type_name) = return_alias_type(document, intent, expression) {
        return Some(type_name);
    }
    let allowed_sources = endpoint_allowed_request_sources(endpoint);
    expression_type(document, intent, expression).or_else(|| {
        collection_expression_type_with_allowed_selectors(
            document,
            intent,
            expression,
            &allowed_sources,
        )
    })
}

fn return_alias_type(document: &RifDocument, intent: &Intent, expression: &str) -> Option<String> {
    let returned = intent
        .returns
        .iter()
        .find(|return_value| return_value.name == expression.trim())?;
    value_expression_type(document, intent, &returned.source)
}

fn check_compensations(steps: &[Step]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut available_outputs = BTreeSet::new();
    for step in steps {
        let mut visible_outputs = available_outputs.clone();
        visible_outputs.extend(step.outputs.keys().cloned());
        if let Some(compensation) = &step.compensation {
            for name in call_reference_names(compensation) {
                if !visible_outputs.contains(&name) {
                    diagnostics.push(
                        Diagnostic::error(
                            "EIGL_COMPENSATION_VALUE_UNAVAILABLE",
                            format!(
                                "Step '{}' compensation '{}' refers to '{}', which is not available on that failure path.",
                                step.title, compensation, name
                            ),
                        )
                        .at(format!("step {}", step.number)),
                    );
                }
            }
        }
        available_outputs.extend(step.outputs.keys().cloned());
    }
    diagnostics
}

fn step_calls(step: &Step) -> Vec<&OperationCall> {
    let mut calls = Vec::new();
    if let Some(call) = &step.call {
        calls.push(call);
    }
    if let Some(call) = &step.otherwise_call {
        calls.push(call);
    }
    calls
}

fn step_invokes(step: &Step) -> Vec<&InvocationTarget> {
    step_invocations(step)
}

fn step_assignments(step: &Step) -> Vec<&String> {
    let mut statements = Vec::new();
    statements.extend(step.set_statements.iter());
    statements.extend(step.otherwise_set_statements.iter());
    statements
}

fn step_appends(step: &Step) -> Vec<&String> {
    let mut statements = Vec::new();
    statements.extend(step.append_statements.iter());
    statements.extend(step.otherwise_append_statements.iter());
    statements
}

fn step_deletes(step: &Step) -> Vec<&String> {
    let mut statements = Vec::new();
    statements.extend(step.delete_statements.iter());
    statements.extend(step.otherwise_delete_statements.iter());
    statements
}

fn step_computes(step: &Step) -> Vec<&String> {
    let mut statements = Vec::new();
    statements.extend(step.compute_statements.iter());
    statements.extend(step.otherwise_compute_statements.iter());
    statements
}

fn failure_is_handled(failure: &str, step: &Step, handlers: &[FailureCase]) -> bool {
    let failure_tokens = tokens(
        &failure
            .replace("Failed", "")
            .replace("Failure", "")
            .replace("Invalid", ""),
    );
    let step_tokens = tokens(&step.title);
    for handler in handlers {
        if handler.stop_failure.as_deref() == Some(failure)
            || handler
                .ignored_failures
                .iter()
                .any(|ignored| ignored == failure)
        {
            return true;
        }
        let condition_tokens = tokens(&handler.condition);
        if !failure_tokens.is_empty() && !failure_tokens.is_disjoint(&condition_tokens) {
            return true;
        }
        if handler.condition.to_ascii_lowercase().contains("fail")
            && !step_tokens.is_disjoint(&condition_tokens)
        {
            return true;
        }
    }
    false
}

fn known_references(document: &RifDocument) -> BTreeSet<String> {
    let mut known: BTreeSet<String> = document.application.things.keys().cloned().collect();
    for thing in document.application.things.values() {
        for field in thing.fields.values() {
            known.insert(format!("{}.{}", thing.name, field.name));
        }
    }
    for intent in &document.intents {
        known.extend(known_references_for_intent(intent));
    }
    known
}

fn endpoint_path_params(path: &str) -> BTreeSet<String> {
    path.split('/')
        .filter_map(|segment| segment.strip_prefix('{')?.strip_suffix('}'))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn expression_type(document: &RifDocument, intent: &Intent, expression: &str) -> Option<String> {
    let mut resolving_local_computes = BTreeSet::new();
    expression_type_inner(document, intent, expression, &mut resolving_local_computes)
}

fn expression_type_inner(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
    resolving_local_computes: &mut BTreeSet<String>,
) -> Option<String> {
    let expression = expression.trim();
    if expression.parse::<i64>().is_ok() {
        return Some("Int".to_string());
    }
    if is_decimal_literal(expression) {
        return Some("Decimal".to_string());
    }
    if is_money_literal(expression) {
        return Some("Money".to_string());
    }
    if is_time_literal(expression) {
        return Some("Time".to_string());
    }
    if is_duration_literal(expression) {
        return Some("Duration".to_string());
    }
    if matches!(expression, "true" | "false") {
        return Some("Bool".to_string());
    }
    if expression.starts_with('"') && expression.ends_with('"') {
        return Some("Text".to_string());
    }
    if let Some(list_type) = list_literal_type(document, intent, expression) {
        return Some(list_type);
    }
    if let Some(list_element_type) = list_lookup_expression_type(document, intent, expression) {
        return Some(list_element_type);
    }
    if let Some(map_type) = map_literal_type(document, intent, expression) {
        return Some(map_type);
    }
    if let Some(map_value_type) = map_lookup_expression_type(document, intent, expression) {
        return Some(map_value_type);
    }
    if let Some(option_type) = option_literal_type(document, intent, expression) {
        return Some(option_type);
    }
    if let Some(result_type) = result_literal_type(document, intent, expression) {
        return Some(result_type);
    }
    if let Some(enum_type) = enum_literal_type(document, expression) {
        return Some(enum_type);
    }

    if let Some(local_type) =
        local_compute_value_type(document, intent, expression, resolving_local_computes)
    {
        return Some(local_type);
    }

    if let Some(iteration_type) = iteration_item_type(document, intent, expression) {
        return Some(iteration_type);
    }

    if let Some(collection_type) = collection_expression_type(document, intent, expression) {
        return Some(collection_type);
    }

    if let Some(result_variant_type) =
        result_variant_projection_type(document, intent, expression, resolving_local_computes)
    {
        return Some(result_variant_type);
    }

    if let Some(option_value_type) =
        option_value_projection_type(document, intent, expression, resolving_local_computes)
    {
        return Some(option_value_type);
    }

    if let Some(object_field_type) =
        object_field_projection_type(document, intent, expression, resolving_local_computes)
    {
        return Some(object_field_type);
    }

    let (root, field_path) = expression.split_once('.').unwrap_or((expression, ""));
    if let Some(thing) = intent
        .subjects
        .get(root)
        .or_else(|| intent.inputs.get(root))
    {
        if field_path.is_empty() {
            return Some(thing.type_name.clone());
        }
        return field_type(document, &thing.type_name, field_path);
    }

    for step in &intent.steps {
        if let Some(output) = step.outputs.get(expression) {
            return Some(output.type_name.clone());
        }
    }
    None
}

fn option_value_projection_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
    resolving_local_computes: &mut BTreeSet<String>,
) -> Option<String> {
    let base = expression.trim().strip_suffix(".value")?.trim();
    if base.is_empty() {
        return None;
    }
    let base_type = expression_type_inner(document, intent, base, resolving_local_computes)?;
    generic_inner(&base_type, "Option").map(ToString::to_string)
}

fn result_variant_projection_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
    resolving_local_computes: &mut BTreeSet<String>,
) -> Option<String> {
    let (base, variant) = expression::split_result_variant_projection(expression)?;
    let base_type = expression_type_inner(document, intent, base, resolving_local_computes)?;
    let result_types = generic_args(&base_type, "Result")?;
    if result_types.len() != 2 {
        return None;
    }
    match variant {
        "success" => Some(result_types[0].to_string()),
        "failure" => Some(result_types[1].to_string()),
        _ => None,
    }
}

fn object_field_projection_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
    resolving_local_computes: &mut BTreeSet<String>,
) -> Option<String> {
    let (base, field_path) = expression::split_last_top_level_dot(expression)?;
    let base_type = expression_type_inner(document, intent, base, resolving_local_computes)?;
    document.application.things.get(&base_type)?;
    field_type(document, &base_type, field_path)
}

fn local_compute_value_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
    resolving_local_computes: &mut BTreeSet<String>,
) -> Option<String> {
    if !is_local_compute_target(expression)
        || !resolving_local_computes.insert(expression.to_string())
    {
        return None;
    }

    let result = intent.steps.iter().find_map(|step| {
        step_computes(step).into_iter().find_map(|statement| {
            let (target, source) = split_top_level_once(statement, '=')?;
            let target = target.trim();
            (target == expression && is_local_compute_target(target)).then(|| {
                arithmetic_expression_type(source.trim(), |operand| {
                    expression_type_inner(document, intent, operand, resolving_local_computes)
                })
            })?
        })
    });

    resolving_local_computes.remove(expression);
    result
}

fn iteration_item_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
) -> Option<String> {
    for step in &intent.steps {
        let item = step.iteration_item.as_deref().unwrap_or("item").trim();
        if item != expression {
            continue;
        }
        let source = step.iterate_over.as_deref()?.trim();
        if source == expression {
            continue;
        }
        let source_type = expression_type(document, intent, source)?;
        if let Some(element_type) = generic_inner(&source_type, "List") {
            return Some(element_type.to_string());
        }
        if let Some(map_types) = generic_args(&source_type, "Map")
            && map_types.len() == 2
        {
            return Some(map_types[0].to_string());
        }
        if collection_expression_type(document, intent, source).is_some() {
            return Some("Text".to_string());
        }
    }
    None
}

fn collection_expression_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
) -> Option<String> {
    collection_expression_type_with_allowed_selectors(
        document,
        intent,
        expression,
        &BTreeSet::new(),
    )
}

fn collection_expression_type_with_allowed_selectors(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
    allowed_selectors: &BTreeSet<String>,
) -> Option<String> {
    let (collection_part, suffix) = split_collection_expression_suffix(expression);
    let (collection_name, selector) = collection_part
        .split_once('[')
        .map_or((collection_part, None), |(name, selector)| {
            (name, Some(selector.trim_end_matches(']')))
        });
    let collection = document.application.collections.get(collection_name)?;
    if let Some(selector) = selector
        && !selector.is_empty()
        && !collection_selector_looks_like_filter(selector)
        && expression_type(document, intent, selector).is_none()
        && !allowed_selectors.contains(selector)
    {
        return None;
    }
    match suffix {
        "" => Some(collection.type_name.clone()),
        "count" => Some("Int".to_string()),
        "keys" => Some("Text".to_string()),
        "keys_json" => Some("List<Text>".to_string()),
        "records" => Some(format!("List<{}>", collection.type_name)),
        "records_json" => Some("List<Map<Text, Text>>".to_string()),
        "record" => Some(collection.type_name.clone()),
        "record_json" => Some("Map<Text, Text>".to_string()),
        "first" => Some(collection.type_name.clone()),
        suffix if suffix.starts_with("first.") => {
            field_type(document, &collection.type_name, &suffix["first.".len()..])
        }
        _ => field_type(document, &collection.type_name, suffix),
    }
}

fn collection_selector_looks_like_filter(selector: &str) -> bool {
    let selector = strip_enclosing_selector_parentheses(selector.trim());
    let disjunctions = split_selector_logical_operator(selector, "or");
    if disjunctions.len() > 1 {
        return disjunctions
            .iter()
            .all(|part| collection_selector_looks_like_filter(part));
    }
    let conjunctions = split_selector_logical_operator(selector, "and");
    if conjunctions.len() > 1 {
        return conjunctions
            .iter()
            .all(|part| collection_selector_looks_like_filter(part));
    }
    if let Some(inner) = selector.strip_prefix("not ")
        && !inner.trim().is_empty()
    {
        return collection_selector_looks_like_filter(inner);
    }
    split_collection_selector_comparison(selector).is_some()
}

fn check_collection_selector_types(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
    context: &str,
    location: Option<String>,
) -> Vec<Diagnostic> {
    let Some((collection_name, selector)) = collection_selector(expression) else {
        return Vec::new();
    };
    let Some(collection) = document.application.collections.get(collection_name) else {
        return Vec::new();
    };
    let selector_context = CollectionSelectorCheckContext {
        document,
        intent,
        collection_name,
        collection_type: &collection.type_name,
        display_expression: expression.trim(),
        context,
        location,
    };
    check_collection_selector_condition_types(&selector_context, selector)
}

struct CollectionSelectorCheckContext<'a> {
    document: &'a RifDocument,
    intent: &'a Intent,
    collection_name: &'a str,
    collection_type: &'a str,
    display_expression: &'a str,
    context: &'a str,
    location: Option<String>,
}

fn check_collection_selector_condition_types(
    selector_context: &CollectionSelectorCheckContext<'_>,
    selector: &str,
) -> Vec<Diagnostic> {
    let selector = strip_enclosing_selector_parentheses(selector.trim());
    let disjunctions = split_selector_logical_operator(selector, "or");
    if disjunctions.len() > 1 {
        return disjunctions
            .into_iter()
            .flat_map(|part| check_collection_selector_condition_types(selector_context, part))
            .collect();
    }
    let conjunctions = split_selector_logical_operator(selector, "and");
    if conjunctions.len() > 1 {
        return conjunctions
            .into_iter()
            .flat_map(|part| check_collection_selector_condition_types(selector_context, part))
            .collect();
    }
    if let Some(inner) = selector.strip_prefix("not ")
        && !inner.trim().is_empty()
    {
        return check_collection_selector_condition_types(selector_context, inner);
    }

    let Some((field, operator, expected)) = split_collection_selector_comparison(selector) else {
        return Vec::new();
    };
    let Some(left_type) = field_type(
        selector_context.document,
        selector_context.collection_type,
        field.trim(),
    ) else {
        let diagnostic = Diagnostic::error(
            "EIGL_UNKNOWN_COLLECTION_SELECTOR_FIELD",
            format!(
                "{} '{}' filters collection '{}' by unknown field '{}'.",
                selector_context.context,
                selector_context.display_expression,
                selector_context.collection_name,
                field.trim()
            ),
        );
        return vec![match &selector_context.location {
            Some(location) => diagnostic.at(location.clone()),
            None => diagnostic,
        }];
    };
    let Some(right_type) = collection_selector_expected_type(
        selector_context.document,
        selector_context.intent,
        &left_type,
        normalized_collection_selector_operator(operator),
        expected.trim(),
    ) else {
        return Vec::new();
    };
    if predicate_operator_types_compatible(
        normalized_collection_selector_operator(operator),
        &left_type,
        &right_type,
    ) {
        return Vec::new();
    }

    let diagnostic = Diagnostic::error(
        "EIGL_COLLECTION_SELECTOR_TYPE",
        format!(
            "{} '{}' filters '{}' ({}) {} '{}' ({}), which is not a valid collection selector operand combination.",
            selector_context.context,
            selector_context.display_expression,
            field.trim(),
            left_type,
            operator,
            expected.trim(),
            right_type
        ),
    );
    vec![match &selector_context.location {
        Some(location) => diagnostic.at(location.clone()),
        None => diagnostic,
    }]
}

fn strip_enclosing_selector_parentheses(mut selector: &str) -> &str {
    loop {
        let trimmed = selector.trim();
        if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
            return trimmed;
        }
        if !outer_selector_parentheses_wrap(trimmed) {
            return trimmed;
        }
        selector = &trimmed[1..trimmed.len() - 1];
    }
}

fn outer_selector_parentheses_wrap(text: &str) -> bool {
    let mut depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
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
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && index != text.len() - 1 && bracket_depth == 0 && brace_depth == 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn split_selector_logical_operator<'a>(selector: &'a str, operator: &str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut in_string = false;
    let mut previous_was_escape = false;

    for (index, ch) in selector.char_indices() {
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
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {
                if paren_depth == 0
                    && bracket_depth == 0
                    && brace_depth == 0
                    && selector[index..].starts_with(operator)
                    && selector_has_logical_boundary(selector, index, index + operator.len())
                {
                    parts.push(selector[start..index].trim());
                    start = index + operator.len();
                }
            }
        }
    }

    if parts.is_empty() {
        vec![selector]
    } else {
        parts.push(selector[start..].trim());
        parts
    }
}

fn selector_has_logical_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(char::is_whitespace) && after.is_none_or(char::is_whitespace)
}

fn collection_selector(expression: &str) -> Option<(&str, &str)> {
    let (collection_part, _) = split_collection_expression_suffix(expression.trim());
    let (collection_name, selector) = collection_part.split_once('[')?;
    Some((collection_name, selector.trim_end_matches(']')))
}

fn collection_selector_expected_type(
    document: &RifDocument,
    intent: &Intent,
    expected_type: &str,
    operator: &str,
    value: &str,
) -> Option<String> {
    if let Some(value_type) = expression_type(document, intent, value) {
        return Some(value_type);
    }
    if let Some(value_type) =
        arithmetic_expression_type(value, |operand| expression_type(document, intent, operand))
    {
        return Some(value_type);
    }
    if matches!(operator, "==" | "!=") {
        if expected_type == "Text"
            && !predicate_operand_looks_like_reference(value)
            && !value.starts_with('"')
        {
            return Some("Text".to_string());
        }
        if state_type_values(expected_type)
            .is_some_and(|states| states.iter().any(|state| state == value))
        {
            return Some(expected_type.to_string());
        }
        if enum_type_values(document, expected_type)
            .is_some_and(|values| values.iter().any(|enum_value| enum_value == value))
        {
            return Some(expected_type.to_string());
        }
    }
    if operator == "contains"
        && let Some(contained_type) = contained_type_for_contains(expected_type)
    {
        if contained_type == "Text"
            && !predicate_operand_looks_like_reference(value)
            && !value.starts_with('"')
        {
            return Some("Text".to_string());
        }
        if state_type_values(&contained_type)
            .is_some_and(|states| states.iter().any(|state| state == value))
        {
            return Some(contained_type);
        }
        if enum_type_values(document, &contained_type)
            .is_some_and(|values| values.iter().any(|enum_value| enum_value == value))
        {
            return Some(contained_type);
        }
    }
    None
}

fn split_collection_selector_comparison(selector: &str) -> Option<(&str, &str, &str)> {
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_string = false;
    let mut previous_was_escape = false;

    for (index, ch) in selector.char_indices() {
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
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            _ if bracket_depth == 0 && brace_depth == 0 && paren_depth == 0 => {
                for operator in [">=", "<=", "==", "!=", ">", "<", "="] {
                    if selector[index..].starts_with(operator) {
                        let left = selector[..index].trim();
                        let right = selector[index + operator.len()..].trim();
                        if left.is_empty() || right.is_empty() {
                            return None;
                        }
                        return Some((left, operator, right));
                    }
                }
                if selector[index..].starts_with("contains")
                    && selector_has_logical_boundary(selector, index, index + "contains".len())
                {
                    let left = selector[..index].trim();
                    let right = selector[index + "contains".len()..].trim();
                    if left.is_empty() || right.is_empty() {
                        return None;
                    }
                    return Some((left, "contains", right));
                }
            }
            _ => {}
        }
    }
    None
}

fn normalized_collection_selector_operator(operator: &str) -> &str {
    if operator == "=" { "==" } else { operator }
}

fn split_collection_expression_suffix(expression: &str) -> (&str, &str) {
    let mut depth = 0usize;
    for (index, ch) in expression.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            '.' if depth == 0 => return (&expression[..index], &expression[index + 1..]),
            _ => {}
        }
    }
    (expression, "")
}

fn field_type(document: &RifDocument, root_type: &str, field_path: &str) -> Option<String> {
    let mut current_type = root_type.to_string();
    for field_name in field_path.split('.') {
        if field_name == "count"
            && (generic_inner(&current_type, "List").is_some()
                || generic_args(&current_type, "Map").is_some())
        {
            current_type = "Int".to_string();
            continue;
        }
        let thing = document.application.things.get(&current_type)?;
        let field = thing.fields.get(field_name)?;
        current_type = field.type_name.clone();
    }
    Some(current_type)
}

fn types_compatible(actual: &str, expected: &str) -> bool {
    let actual = actual.trim();
    let expected = expected.trim();
    if actual == expected || (actual == "Int" && expected == "Decimal") {
        return true;
    }
    if let (Some(actual_inner), Some(expected_inner)) = (
        generic_inner(actual, "List"),
        generic_inner(expected, "List"),
    ) {
        return types_compatible(actual_inner, expected_inner);
    }
    if let (Some(actual_args), Some(expected_args)) =
        (generic_args(actual, "Map"), generic_args(expected, "Map"))
        && actual_args.len() == 2
        && expected_args.len() == 2
    {
        let key_matches =
            actual_args[0] == "Unit" || types_compatible(actual_args[0], expected_args[0]);
        let value_matches =
            actual_args[1] == "Unit" || types_compatible(actual_args[1], expected_args[1]);
        return key_matches && value_matches;
    }
    if let (Some(actual_inner), Some(expected_inner)) = (
        generic_inner(actual, "Option"),
        generic_inner(expected, "Option"),
    ) {
        return actual_inner == "Unit" || types_compatible(actual_inner, expected_inner);
    }
    if let (Some(actual_inner), Some(expected_inner)) = (
        generic_inner(actual, "Secret"),
        generic_inner(expected, "Secret"),
    ) {
        return types_compatible(actual_inner, expected_inner);
    }
    if let (Some(actual_args), Some(expected_args)) = (
        generic_args(actual, "Result"),
        generic_args(expected, "Result"),
    ) && actual_args.len() == 2
        && expected_args.len() == 2
    {
        let success_matches =
            actual_args[0] == "Unit" || types_compatible(actual_args[0], expected_args[0]);
        let failure_matches =
            actual_args[1] == "Unit" || types_compatible(actual_args[1], expected_args[1]);
        return success_matches && failure_matches;
    }
    false
}

fn type_contains_secret(document: &RifDocument, type_name: &str) -> bool {
    let mut seen_types = BTreeSet::new();
    type_contains_secret_inner(document, type_name, &mut seen_types)
}

fn type_contains_secret_inner(
    document: &RifDocument,
    type_name: &str,
    seen_types: &mut BTreeSet<String>,
) -> bool {
    let type_name = type_name.trim();
    if generic_inner(type_name, "Secret").is_some() {
        return true;
    }
    if let Some(thing) = document.application.things.get(type_name) {
        if !seen_types.insert(type_name.to_string()) {
            return false;
        }
        return thing.fields.values().any(|field| {
            field.is_secret || type_contains_secret_inner(document, &field.type_name, seen_types)
        });
    }
    let Some((_, inner)) = type_name.split_once('<') else {
        return false;
    };
    let Some(inner) = inner.strip_suffix('>') else {
        return false;
    };
    split_top_level_commas(inner)
        .into_iter()
        .any(|inner_type| type_contains_secret_inner(document, inner_type, seen_types))
}

fn is_decimal_literal(value: &str) -> bool {
    value.contains('.') && value.parse::<f64>().is_ok_and(f64::is_finite)
}

fn is_money_literal(value: &str) -> bool {
    let Some((currency, amount)) = value.split_once(':') else {
        return false;
    };
    currency.len() == 3
        && currency.chars().all(|ch| ch.is_ascii_uppercase())
        && amount.parse::<f64>().is_ok_and(f64::is_finite)
}

fn is_time_literal(value: &str) -> bool {
    if value.len() != 20 {
        return false;
    }
    let bytes = value.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .filter(|(index, _)| !matches!(index, 4 | 7 | 10 | 13 | 16 | 19))
        .all(|(_, byte)| byte.is_ascii_digit())
    {
        return false;
    }

    let month = parse_time_part(value, 5, 7);
    let day = parse_time_part(value, 8, 10);
    let hour = parse_time_part(value, 11, 13);
    let minute = parse_time_part(value, 14, 16);
    let second = parse_time_part(value, 17, 19);
    matches!(month, Some(1..=12))
        && matches!(day, Some(1..=31))
        && matches!(hour, Some(0..=23))
        && matches!(minute, Some(0..=59))
        && matches!(second, Some(0..=59))
}

fn parse_time_part(value: &str, start: usize, end: usize) -> Option<u32> {
    value.get(start..end)?.parse().ok()
}

fn is_duration_literal(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('P') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }

    let mut chars = rest.chars().peekable();
    let mut in_time = false;
    let mut seen_time_marker = false;
    let mut seen_component = false;
    while chars.peek().is_some() {
        if chars.peek() == Some(&'T') {
            if seen_time_marker {
                return false;
            }
            seen_time_marker = true;
            in_time = true;
            chars.next();
            if chars.peek().is_none() {
                return false;
            }
            continue;
        }

        let mut has_digits = false;
        while chars.peek().is_some_and(char::is_ascii_digit) {
            has_digits = true;
            chars.next();
        }
        if !has_digits {
            return false;
        }

        let Some(unit) = chars.next() else {
            return false;
        };
        let valid_unit = if in_time {
            matches!(unit, 'H' | 'M' | 'S')
        } else {
            matches!(unit, 'Y' | 'M' | 'W' | 'D')
        };
        if !valid_unit {
            return false;
        }
        seen_component = true;
    }
    seen_component
}

fn list_literal_type(document: &RifDocument, intent: &Intent, expression: &str) -> Option<String> {
    let items = list_literal_items(expression)?;
    if items.is_empty() {
        return Some("List<Unit>".to_string());
    }

    let mut item_types = items
        .iter()
        .map(|item| expression_type(document, intent, item));
    let mut element_type = item_types.next()??;
    for item_type in item_types {
        let item_type = item_type?;
        if types_compatible(&item_type, &element_type) {
            continue;
        }
        if types_compatible(&element_type, &item_type) {
            element_type = item_type;
            continue;
        }
        return None;
    }
    Some(format!("List<{element_type}>"))
}

fn list_literal_items(expression: &str) -> Option<Vec<&str>> {
    let inner = expression.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        split_top_level_commas(inner)
            .into_iter()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn list_lookup_expression_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
) -> Option<String> {
    let (list_expression, index_expression) = expression::split_index_lookup(expression)?;
    let list_type = expression_type(document, intent, list_expression)?;
    let element_type = generic_inner(&list_type, "List")?;
    let index_type = value_expression_type(document, intent, index_expression)?;
    types_compatible(&index_type, "Int").then(|| element_type.to_string())
}

fn map_literal_type(document: &RifDocument, intent: &Intent, expression: &str) -> Option<String> {
    let entries = map_literal_entries(expression)?;
    if entries.is_empty() {
        return Some("Map<Unit, Unit>".to_string());
    }

    let mut entry_types = entries.iter().map(|(key, value)| {
        Some((
            expression_type(document, intent, key)?,
            expression_type(document, intent, value)?,
        ))
    });
    let (mut key_type, mut value_type) = entry_types.next()??;
    for entry_type in entry_types {
        let (entry_key_type, entry_value_type) = entry_type?;
        if types_compatible(&entry_key_type, &key_type) {
        } else if types_compatible(&key_type, &entry_key_type) {
            key_type = entry_key_type;
        } else {
            return None;
        }

        if types_compatible(&entry_value_type, &value_type) {
        } else if types_compatible(&value_type, &entry_value_type) {
            value_type = entry_value_type;
        } else {
            return None;
        }
    }
    Some(format!("Map<{key_type}, {value_type}>"))
}

fn map_literal_entries(expression: &str) -> Option<Vec<(&str, &str)>> {
    let inner = expression.strip_prefix('{')?.strip_suffix('}')?;
    split_top_level_commas(inner)
        .into_iter()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| split_top_level_once(entry, ':'))
        .collect()
}

fn map_lookup_expression_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
) -> Option<String> {
    let (map_expression, key_expression) = expression::split_map_lookup(expression)?;
    let map_type = expression_type(document, intent, map_expression)?;
    let map_types = generic_args(&map_type, "Map")?;
    if map_types.len() != 2 {
        return None;
    }

    let key_type = value_expression_type(document, intent, key_expression).or_else(|| {
        (map_types[0] == "Text" && plain_text_map_key_literal(key_expression))
            .then(|| "Text".to_string())
    })?;
    if !types_compatible(&key_type, map_types[0]) {
        return None;
    }

    Some(map_types[1].to_string())
}

fn plain_text_map_key_literal(expression: &str) -> bool {
    let expression = expression.trim();
    !expression.is_empty()
        && expression
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '/' | ' '))
        && expression.chars().any(|ch| !ch.is_ascii_digit())
}

fn option_literal_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
) -> Option<String> {
    if expression == "None" {
        return Some("Option<Unit>".to_string());
    }

    let inner = expression.strip_prefix("Some(")?.strip_suffix(')')?.trim();
    let inner_type = expression_type(document, intent, inner)?;
    Some(format!("Option<{inner_type}>"))
}

fn result_literal_type(
    document: &RifDocument,
    intent: &Intent,
    expression: &str,
) -> Option<String> {
    if let Some(inner) = constructor_value(expression, "Success") {
        let inner_type = expression_type(document, intent, inner)?;
        return Some(format!("Result<{inner_type}, Unit>"));
    }
    if let Some(inner) = constructor_value(expression, "Failure") {
        let inner_type = expression_type(document, intent, inner)?;
        return Some(format!("Result<Unit, {inner_type}>"));
    }
    None
}

fn constructor_value<'a>(expression: &'a str, constructor: &str) -> Option<&'a str> {
    let prefix = format!("{constructor}(");
    expression
        .strip_prefix(&prefix)?
        .strip_suffix(')')
        .map(str::trim)
}

fn generic_inner<'a>(type_name: &'a str, wrapper: &str) -> Option<&'a str> {
    let trimmed = type_name.trim();
    let inner = trimmed.strip_prefix(wrapper)?.trim();
    inner.strip_prefix('<')?.strip_suffix('>').map(str::trim)
}

fn generic_args<'a>(type_name: &'a str, wrapper: &str) -> Option<Vec<&'a str>> {
    Some(
        split_top_level_commas(generic_inner(type_name, wrapper)?)
            .into_iter()
            .map(str::trim)
            .collect(),
    )
}

fn is_local_compute_target(value: &str) -> bool {
    let mut chars = value.trim().chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
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

fn split_top_level_once(text: &str, separator: char) -> Option<(&str, &str)> {
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
            ch if ch == separator
                && angle_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && paren_depth == 0 =>
            {
                let right_start = index + ch.len_utf8();
                return Some((text[..index].trim(), text[right_start..].trim()));
            }
            _ => {}
        }
    }
    None
}

fn enum_type_values(document: &RifDocument, type_name: &str) -> Option<Vec<String>> {
    document
        .application
        .enums
        .get(type_name.trim())
        .map(|definition| definition.values.clone())
}

fn enum_literal_type(document: &RifDocument, value: &str) -> Option<String> {
    let mut matches = document.application.enums.values().filter(|definition| {
        definition
            .values
            .iter()
            .any(|enum_value| enum_value == value)
    });
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(first.name.clone())
}

fn state_type_values(type_name: &str) -> Option<Vec<String>> {
    let inner = type_name.trim().strip_prefix("State<")?.strip_suffix('>')?;
    Some(
        inner
            .split(',')
            .map(str::trim)
            .filter(|state| !state.is_empty())
            .map(ToString::to_string)
            .collect(),
    )
}

fn known_references_for_intent(intent: &crate::rif_model::Intent) -> BTreeSet<String> {
    let mut known: BTreeSet<String> = intent
        .subjects
        .keys()
        .chain(intent.inputs.keys())
        .cloned()
        .collect();
    for requirement in &intent.requires {
        if let Some(reference) = first_reference(&requirement.text) {
            known.insert(reference.clone());
            known.insert(root_name(&reference).to_string());
        }
        if let Some(state) = state_after_is(&requirement.text) {
            known.insert(state);
        }
    }
    for transition in &intent.state_transitions {
        known.insert(transition.field_path.clone());
        known.insert(root_name(&transition.field_path).to_string());
        known.insert(transition.from_state.clone());
        known.insert(transition.to_state.clone());
    }
    for step in &intent.steps {
        if step.iterate_over.is_some() {
            known.insert(
                step.iteration_item
                    .clone()
                    .unwrap_or_else(|| "item".to_string()),
            );
        }
        known.extend(step.outputs.keys().cloned());
        for target in step.reads.iter().chain(step.changes.iter()) {
            known.insert(target.clone());
            known.insert(root_name(target).to_string());
        }
        for statement in step
            .set_statements
            .iter()
            .chain(step.otherwise_set_statements.iter())
        {
            if let Some((left, right)) = split_top_level_once(statement, '=') {
                known.insert(left.trim().to_string());
                known.insert(root_name(left.trim()).to_string());
                known.insert(right.trim().to_string());
            } else {
                known.insert(statement.trim().to_string());
                known.insert(root_name(statement.trim()).to_string());
            }
        }
        for statement in step
            .append_statements
            .iter()
            .chain(step.otherwise_append_statements.iter())
        {
            if let Some((left, right)) = statement.split_once("+=") {
                known.insert(left.trim().to_string());
                known.insert(root_name(left.trim()).to_string());
                known.insert(right.trim().to_string());
            } else {
                known.insert(statement.trim().to_string());
                known.insert(root_name(statement.trim()).to_string());
            }
        }
        for statement in step
            .compute_statements
            .iter()
            .chain(step.otherwise_compute_statements.iter())
        {
            if let Some((left, _)) = split_top_level_once(statement, '=') {
                let left = left.trim();
                if is_local_compute_target(left) {
                    known.insert(left.to_string());
                }
            }
        }
        for statement in step
            .delete_statements
            .iter()
            .chain(step.otherwise_delete_statements.iter())
        {
            known.insert(statement.trim().to_string());
            known.insert(collection_aware_root_name(statement.trim()).to_string());
        }
        if let Some(call) = &step.call {
            for arg in &call.args {
                known.insert(arg.clone());
                known.insert(root_name(arg).to_string());
            }
        }
    }
    for returned in &intent.returns {
        known.insert(returned.name.clone());
    }
    known
}

fn known_types(document: &RifDocument) -> BTreeSet<String> {
    let mut types: BTreeSet<String> = [
        "Bool", "Text", "Unit", "Int", "Decimal", "Money", "Time", "Duration",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect();
    types.extend(document.application.enums.keys().cloned());
    types.extend(document.application.things.keys().cloned());
    for intent in &document.intents {
        types.extend(
            intent
                .subjects
                .values()
                .map(|thing| thing.type_name.clone()),
        );
        types.extend(intent.inputs.values().map(|thing| thing.type_name.clone()));
    }
    types
}

fn referenced_types(type_name: &str) -> Vec<String> {
    let mut result = Vec::new();
    collect_type_names(type_name, &mut result);
    result
}

fn collect_type_names(type_name: &str, result: &mut Vec<String>) {
    let trimmed = type_name.trim();
    if let Some((outer, inner)) = trimmed.split_once('<') {
        let outer = outer.trim();
        if outer == "State" {
            return;
        }
        if !is_generic_wrapper(outer) {
            result.push(outer.to_string());
        }
        let Some(inner) = inner.trim().strip_suffix('>') else {
            result.push(trimmed.to_string());
            return;
        };
        for part in split_top_level_commas(inner) {
            collect_type_names(part, result);
        }
    } else if !trimmed.is_empty() {
        result.push(trimmed.to_string());
    }
}

fn is_generic_wrapper(name: &str) -> bool {
    matches!(
        name,
        "Id" | "Ref" | "List" | "Array" | "Set" | "Map" | "Option" | "Result" | "Secret"
    )
}

fn first_reference(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?.trim_matches(':');
    token
        .chars()
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        .then(|| token.to_string())
}

fn state_after_is(text: &str) -> Option<String> {
    let parts: Vec<_> = text.split_whitespace().collect();
    let value = parts
        .windows(2)
        .find(|window| window[0] == "is")
        .map(|window| window[1])?;
    if predicate_operand_looks_like_reference(value) {
        None
    } else {
        Some(value.to_string())
    }
}

fn predicate_operand_looks_like_reference(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.parse::<f64>().is_err()
        && !value.starts_with(['"', '{', '['])
        && !value.contains(':')
        && (value.contains('.') || value.contains('['))
}

fn root_name(reference: &str) -> &str {
    reference
        .split_once('.')
        .map_or(reference, |(root, _)| root)
}

fn tokens(text: &str) -> BTreeSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphabetic())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| !matches!(token.as_str(), "with" | "failed" | "fails" | "failure"))
        .collect()
}

fn call_reference_names(expression: &str) -> Vec<String> {
    let Some((_, rest)) = expression.split_once('(') else {
        return Vec::new();
    };
    let args = rest.rsplit_once(')').map_or(rest, |(args, _)| args);
    args.split(',')
        .map(str::trim)
        .filter(|arg| {
            arg.chars()
                .next()
                .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
                && arg
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        })
        .map(ToString::to_string)
        .collect()
}
