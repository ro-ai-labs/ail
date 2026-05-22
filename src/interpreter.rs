use std::collections::BTreeMap;

use crate::collections::{
    collection_path_value_with, collection_query_keys_with, delete_collection_path_with,
};
use crate::core_model::{json_string, json_value};
use crate::expression;
use crate::predicate;
use crate::rif_model::{FailureCase, InvocationTarget, RifDocument, Step};

const LOOP_LIMIT: usize = 1024;
type ParallelJoin = (
    BTreeMap<String, String>,
    BTreeMap<String, String>,
    Vec<String>,
);

struct ExecutionControls<'a> {
    operation_outputs: &'a BTreeMap<String, String>,
    fail_at: &'a BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationResult {
    pub status: String,
    pub final_state: BTreeMap<String, String>,
    pub outputs: BTreeMap<String, String>,
    pub trace: Vec<String>,
    pub failure: Option<String>,
}

pub fn simulate(
    document: &RifDocument,
    initial_state: BTreeMap<String, String>,
    fail_at: BTreeMap<String, String>,
) -> SimulationResult {
    simulate_with_operation_outputs(document, initial_state, BTreeMap::new(), fail_at)
}

pub fn simulate_with_operation_outputs(
    document: &RifDocument,
    initial_state: BTreeMap<String, String>,
    operation_outputs: BTreeMap<String, String>,
    fail_at: BTreeMap<String, String>,
) -> SimulationResult {
    let mut state = initial_state;
    let mut outputs = BTreeMap::new();
    let mut trace = Vec::new();
    let controls = ExecutionControls {
        operation_outputs: &operation_outputs,
        fail_at: &fail_at,
    };

    for requirement in &document.intent.requires {
        if evaluate_predicate(&requirement.text, &state, &outputs) {
            trace.push(format!("CHECK {}", requirement.text));
        } else {
            trace.push(format!("CHECK FAILED {}", requirement.text));
            return SimulationResult {
                status: "failed".to_string(),
                final_state: state,
                outputs,
                trace,
                failure: Some("RequirementFailed".to_string()),
            };
        }
    }

    for step in &document.intent.steps {
        let mut iterations = 0usize;
        loop {
            let guard_passed = step
                .guard
                .as_ref()
                .is_none_or(|guard| evaluate_predicate(guard, &state, &outputs));
            let use_else = !guard_passed && has_else_branch(step);
            if !guard_passed && !use_else {
                trace.push(format!(
                    "SKIP {} when {}",
                    step.title,
                    step.guard.as_deref().unwrap_or("")
                ));
                break;
            }

            trace.push(step.title.clone());
            if let Some(query) = &step.iterate_over {
                let item_name = step.iteration_item.as_deref().unwrap_or("item").to_string();
                let iterated_items = iteration_items(query, &state, &outputs);
                trace.push(format!("iterate {query}"));
                for item in iterated_items {
                    state.insert(item_name.clone(), item);
                    if let Some(result) = execute_step_body(
                        document,
                        step,
                        use_else,
                        &mut state,
                        &mut outputs,
                        &mut trace,
                        &controls,
                    ) {
                        return result;
                    }
                }
            } else if let Some(result) = execute_step_body(
                document,
                step,
                use_else,
                &mut state,
                &mut outputs,
                &mut trace,
                &controls,
            ) {
                return result;
            }

            let should_repeat = if let Some(condition) = &step.repeat_while {
                evaluate_predicate(condition, &state, &outputs)
            } else if let Some(condition) = &step.repeat_until {
                !evaluate_predicate(condition, &state, &outputs)
            } else {
                false
            };
            if should_repeat {
                iterations += 1;
                if iterations > LOOP_LIMIT {
                    return SimulationResult {
                        status: "failed".to_string(),
                        final_state: state,
                        outputs,
                        trace,
                        failure: Some("LoopLimitExceeded".to_string()),
                    };
                }
                continue;
            }
            break;
        }
    }

    publish_returns(document, &state, &mut outputs);
    SimulationResult {
        status: "succeeded".to_string(),
        final_state: state,
        outputs,
        trace,
        failure: None,
    }
}

fn iteration_items(
    query: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Vec<String> {
    let collection_items = collection_query_keys_with(state, query, |name| {
        outputs
            .get(name)
            .cloned()
            .or_else(|| state.get(name).cloned())
    });
    if !collection_items.is_empty() {
        return collection_items;
    }

    let value = evaluate_expression(query, state, outputs);
    expression::list_literal_values(&value)
        .or_else(|| expression::map_literal_keys(&value))
        .unwrap_or_default()
}

fn execute_step_body(
    document: &RifDocument,
    step: &Step,
    use_else: bool,
    state: &mut BTreeMap<String, String>,
    outputs: &mut BTreeMap<String, String>,
    trace: &mut Vec<String>,
    controls: &ExecutionControls<'_>,
) -> Option<SimulationResult> {
    let call = if use_else {
        step.otherwise_call.as_ref()
    } else {
        step.call.as_ref()
    };
    if let Some(failure) =
        call.and_then(|call| forced_failure(step, &call.target, controls.fail_at))
    {
        return Some(apply_failure(
            document,
            step,
            &failure,
            state.clone(),
            outputs.clone(),
            trace.clone(),
        ));
    }

    for output in step.outputs.keys() {
        outputs.insert(
            output.clone(),
            controls
                .operation_outputs
                .get(output)
                .cloned()
                .unwrap_or_else(|| format!("{output}#{}", step.number)),
        );
        trace.push(format!("produce {output}"));
    }

    let set_statements = if use_else {
        &step.otherwise_set_statements
    } else {
        &step.set_statements
    };
    for statement in set_statements {
        apply_set(document, statement, state, outputs);
        trace.push(format!("set {statement}"));
    }

    let append_statements = if use_else {
        &step.otherwise_append_statements
    } else {
        &step.append_statements
    };
    for statement in append_statements {
        apply_append(document, statement, state, outputs);
        trace.push(format!("append {statement}"));
    }

    let compute_statements = if use_else {
        &step.otherwise_compute_statements
    } else {
        &step.compute_statements
    };
    for statement in compute_statements {
        apply_compute(document, statement, state, outputs);
        trace.push(format!("compute {statement}"));
    }

    let delete_statements = if use_else {
        &step.otherwise_delete_statements
    } else {
        &step.delete_statements
    };
    for statement in delete_statements {
        apply_delete(statement, state, outputs);
        trace.push(format!("delete {statement}"));
    }

    let invoke_target = if use_else {
        step.otherwise_invoke.as_ref()
    } else {
        step.invoke.as_ref()
    };
    if let Some(target) = invoke_target {
        trace.push(format!("invoke {}", target.target));
        let Some(subdocument) = subdocument_for_intent(document, &target.target) else {
            return Some(SimulationResult {
                status: "failed".to_string(),
                final_state: state.clone(),
                outputs: outputs.clone(),
                trace: trace.clone(),
                failure: Some("UnknownIntent".to_string()),
            });
        };
        let mut child_state = state.clone();
        apply_invocation_bindings(target, document, state, outputs, &mut child_state);
        let mut result = simulate_with_operation_outputs(
            &subdocument,
            child_state,
            controls.operation_outputs.clone(),
            controls.fail_at.clone(),
        );
        trace.extend(result.trace);
        if result.status == "failed" {
            return Some(SimulationResult {
                status: "failed".to_string(),
                final_state: result.final_state,
                outputs: result.outputs,
                trace: trace.clone(),
                failure: result.failure,
            });
        }
        apply_invocation_result_bindings(target, document, state, outputs, &mut result.final_state);
        *state = result.final_state;
        outputs.extend(result.outputs);
    }

    let parallel_targets = if use_else {
        &step.otherwise_parallel_invokes
    } else {
        &step.parallel_invokes
    };
    if !parallel_targets.is_empty() {
        trace.push(format!(
            "fanout {}",
            parallel_targets
                .iter()
                .map(|target| target.target.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ));
        match run_parallel_invocations(
            document,
            parallel_targets,
            state.clone(),
            outputs.clone(),
            controls.operation_outputs.clone(),
            controls.fail_at.clone(),
        ) {
            Ok((merged_state, merged_outputs, parallel_trace)) => {
                trace.extend(parallel_trace);
                *state = merged_state;
                *outputs = merged_outputs;
            }
            Err(result) => return Some(result),
        }
    }

    None
}

fn publish_returns(
    document: &crate::rif_model::RifDocument,
    state: &BTreeMap<String, String>,
    outputs: &mut BTreeMap<String, String>,
) {
    for return_value in &document.intent.returns {
        let output_snapshot = outputs.clone();
        let value = typed_object_return_value(document, &return_value.source, state)
            .unwrap_or_else(|| {
                evaluate_typed_expression(document, &return_value.source, state, &output_snapshot)
            });
        outputs.insert(return_value.name.clone(), value);
    }
}

fn typed_object_return_value(
    document: &crate::rif_model::RifDocument,
    source: &str,
    state: &BTreeMap<String, String>,
) -> Option<String> {
    let source = source.trim();
    let type_name = object_source_type(document, source)?;
    let thing = document.application.things.get(&type_name)?;
    typed_object_state_value(document, source, thing, state)
}

fn typed_object_expression_value(
    document: &crate::rif_model::RifDocument,
    source: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let source = source.trim();
    let type_name = object_source_type(document, source)?;
    let thing = document.application.things.get(&type_name)?;
    typed_object_state_value_with_outputs(document, source, thing, state, outputs)
}

fn object_source_type(document: &crate::rif_model::RifDocument, source: &str) -> Option<String> {
    let (root, field_path) = split_object_path_suffix(source);
    if let Some(collection_type) = collection_record_type(document, root) {
        return if field_path.is_empty() {
            Some(collection_type)
        } else {
            object_field_type(document, &collection_type, field_path)
        };
    }
    if let Some(output_type) = operation_output_source_type(document, root) {
        return if field_path.is_empty() {
            Some(output_type)
        } else {
            object_field_type(document, &output_type, field_path)
        };
    }
    let root_type = document
        .intent
        .subjects
        .get(root)
        .or_else(|| document.intent.inputs.get(root))?
        .type_name
        .clone();
    if field_path.is_empty() {
        Some(root_type)
    } else {
        object_field_type(document, &root_type, field_path)
    }
}

fn collection_record_type(document: &crate::rif_model::RifDocument, path: &str) -> Option<String> {
    let (collection_name, _) = path.trim().split_once('[')?;
    document
        .application
        .collections
        .get(collection_name)
        .map(|collection| collection.type_name.clone())
}

fn operation_output_source_type(
    document: &crate::rif_model::RifDocument,
    output_name: &str,
) -> Option<String> {
    document
        .intent
        .steps
        .iter()
        .find_map(|step| step.outputs.get(output_name))
        .map(|output| output.type_name.clone())
}

fn split_object_path_suffix(path: &str) -> (&str, &str) {
    let mut depth = 0usize;
    for (index, ch) in path.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            '.' if depth == 0 => return (&path[..index], &path[index + 1..]),
            _ => {}
        }
    }
    (path, "")
}

fn object_field_type(
    document: &crate::rif_model::RifDocument,
    root_type: &str,
    field_path: &str,
) -> Option<String> {
    let mut current_type = root_type.to_string();
    for field_name in field_path.split('.') {
        let thing = document.application.things.get(&current_type)?;
        let field = thing.fields.get(field_name)?;
        current_type = field.type_name.clone();
    }
    Some(current_type)
}

fn typed_object_state_value(
    document: &crate::rif_model::RifDocument,
    source: &str,
    thing: &crate::rif_model::ThingDefinition,
    state: &BTreeMap<String, String>,
) -> Option<String> {
    let mut entries = Vec::new();
    for field in thing.fields.values() {
        if field.is_secret {
            return None;
        }
        let field_source = format!("{source}.{}", field.name);
        let value = if let Some(nested_thing) = document.application.things.get(&field.type_name) {
            typed_object_state_value(document, &field_source, nested_thing, state)?
        } else {
            state.get(&field_source)?.clone()
        };
        entries.push(format!(
            "{}:{}",
            json_string(&field.name),
            json_value(&value)
        ));
    }
    Some(format!("{{{}}}", entries.join(",")))
}

fn typed_object_state_value_with_outputs(
    document: &crate::rif_model::RifDocument,
    source: &str,
    thing: &crate::rif_model::ThingDefinition,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let mut entries = Vec::new();
    for field in thing.fields.values() {
        if field.is_secret {
            return None;
        }
        let field_source = format!("{source}.{}", field.name);
        let value = if let Some(nested_thing) = document.application.things.get(&field.type_name) {
            typed_object_state_value_with_outputs(
                document,
                &field_source,
                nested_thing,
                state,
                outputs,
            )?
        } else {
            runtime_value_with_outputs(&field_source, state, outputs)?
        };
        entries.push(format!(
            "{}:{}",
            json_string(&field.name),
            json_value(&value)
        ));
    }
    Some(format!("{{{}}}", entries.join(",")))
}

fn runtime_value_with_outputs(
    source: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let resolved_source = resolve_state_path(source, state, outputs);
    state
        .get(&resolved_source)
        .or_else(|| outputs.get(&resolved_source))
        .cloned()
        .or_else(|| {
            expression::resolve_object_field_lookup(source, |name| {
                runtime_value_with_outputs(name, state, outputs)
            })
        })
}

fn apply_failure(
    document: &RifDocument,
    step: &Step,
    failure: &str,
    mut state: BTreeMap<String, String>,
    outputs: BTreeMap<String, String>,
    mut trace: Vec<String>,
) -> SimulationResult {
    let mut stop_failure = failure.to_string();
    if let Some(handler) = find_handler(&document.intent.failure_handlers, step, failure) {
        for action in &handler.actions {
            if let Some(statement) = action.strip_prefix("set ") {
                apply_set(document, statement.trim(), &mut state, &outputs);
                trace.push(action.clone());
            } else if let Some(stop) = action.strip_prefix("stop with ") {
                stop_failure = stop.trim().to_string();
                trace.push(action.clone());
            } else {
                trace.push(action.clone());
            }
        }
    }
    SimulationResult {
        status: "failed".to_string(),
        final_state: state,
        outputs,
        trace,
        failure: Some(stop_failure),
    }
}

fn forced_failure(step: &Step, target: &str, fail_at: &BTreeMap<String, String>) -> Option<String> {
    fail_at
        .get(&step.title)
        .cloned()
        .or_else(|| fail_at.get(target).cloned())
}

fn find_handler<'a>(
    handlers: &'a [FailureCase],
    step: &Step,
    failure: &str,
) -> Option<&'a FailureCase> {
    let failure_token = failure
        .to_ascii_lowercase()
        .replace("failed", "")
        .replace("failure", "");
    let step_tokens = tokens(&step.title);
    handlers.iter().find(|handler| {
        handler.stop_failure.as_deref() == Some(failure)
            || handler
                .ignored_failures
                .iter()
                .any(|ignored| ignored == failure)
            || (!failure_token.is_empty()
                && handler
                    .condition
                    .to_ascii_lowercase()
                    .contains(failure_token.trim()))
            || (!step_tokens.is_empty()
                && handler.condition.to_ascii_lowercase().contains("fail")
                && !step_tokens
                    .iter()
                    .all(|token| !handler.condition.to_ascii_lowercase().contains(token)))
    })
}

fn apply_set(
    document: &crate::rif_model::RifDocument,
    statement: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) {
    let Some((left, right)) = statement.split_once('=') else {
        return;
    };
    let right = right.trim();
    if apply_typed_object_set(document, left.trim(), right, state, outputs).is_some() {
        return;
    }
    let value = evaluate_typed_expression(document, right, state, outputs);
    if apply_container_option_value_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_container_result_variant_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_container_option_object_field_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_container_result_object_field_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_option_value_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_result_variant_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_option_object_field_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_result_object_field_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_option_container_object_field_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_result_container_object_field_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_option_container_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_result_container_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_object_field_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_map_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    if apply_list_entry_set(left.trim(), &value, state, outputs) {
        return;
    }
    let key = resolve_state_path(left.trim(), state, outputs);
    state.insert(key, value);
}

fn apply_typed_object_set(
    document: &crate::rif_model::RifDocument,
    target: &str,
    source: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<Vec<String>> {
    let target = target.trim();
    let source = source.trim();
    let target_type = object_source_type(document, target)?;
    let source_type = object_source_type(document, source)?;
    if target_type != source_type {
        return None;
    }
    let thing = document.application.things.get(&target_type)?;
    copy_typed_object_fields(document, target, source, thing, state, outputs)
}

fn copy_typed_object_fields(
    document: &crate::rif_model::RifDocument,
    target: &str,
    source: &str,
    thing: &crate::rif_model::ThingDefinition,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<Vec<String>> {
    let mut changed_fields = Vec::new();
    for field in thing.fields.values() {
        let target_field = format!("{target}.{}", field.name);
        let source_field = format!("{source}.{}", field.name);
        if let Some(nested_thing) = document.application.things.get(&field.type_name) {
            changed_fields.extend(copy_typed_object_fields(
                document,
                &target_field,
                &source_field,
                nested_thing,
                state,
                outputs,
            )?);
        } else {
            let value = runtime_value_with_outputs(&source_field, state, outputs)?;
            let resolved_target = resolve_state_path(&target_field, state, outputs);
            state.insert(resolved_target.clone(), value);
            changed_fields.push(resolved_target);
        }
    }
    Some(changed_fields)
}

fn apply_compute(
    document: &crate::rif_model::RifDocument,
    statement: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &mut BTreeMap<String, String>,
) {
    let Some((left, right)) = statement.split_once('=') else {
        return;
    };
    let value = evaluate_typed_expression(document, right.trim(), state, outputs);
    let target = left.trim();
    if is_local_compute_target(target) {
        outputs.insert(target.to_string(), value);
    } else {
        let key = resolve_state_path(target, state, outputs);
        state.insert(key, value);
    }
}

fn apply_append(
    document: &crate::rif_model::RifDocument,
    statement: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) {
    let Some((left, right)) = statement.split_once("+=") else {
        return;
    };
    let target = left.trim();
    let right = right.trim();
    let value = evaluate_typed_expression(document, right, state, outputs);
    if apply_container_option_list_append(target, &value, state, outputs) {
        return;
    }
    if apply_container_result_list_append(target, &value, state, outputs) {
        return;
    }
    if apply_option_list_append(target, &value, state, outputs) {
        return;
    }
    if apply_result_list_append(target, &value, state, outputs) {
        return;
    }
    let key = resolve_state_path(target, state, outputs);
    let current = state.get(&key).map(String::as_str);
    state.insert(key, append_list_value(current, &value));
}

fn append_list_value(current: Option<&str>, value: &str) -> String {
    let Some(current) = current.map(str::trim).filter(|value| !value.is_empty()) else {
        return format!("[{value}]");
    };
    let Some(inner) = current
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
    else {
        return format!("[{current},{value}]");
    };
    if inner.trim().is_empty() {
        format!("[{value}]")
    } else {
        format!("[{inner},{value}]")
    }
}

fn is_local_compute_target(value: &str) -> bool {
    let mut chars = value.trim().chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn apply_map_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((map_expression, key_expression)) = expression::split_map_lookup(target) else {
        return false;
    };
    let map_key = resolve_state_path(map_expression, state, outputs);
    let Some(current_map) = state
        .get(&map_key)
        .cloned()
        .or_else(|| outputs.get(&map_key).cloned())
    else {
        return false;
    };
    let lookup_key = evaluate_expression(key_expression, state, outputs);
    let Some(updated_map) = expression::set_map_lookup_value(&current_map, &lookup_key, value)
    else {
        return false;
    };
    state.insert(map_key, updated_map);
    true
}

fn apply_container_option_value_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some(entry_expression) = target.trim().strip_suffix(".value").map(str::trim) else {
        return false;
    };
    let Some((container_expression, index_expression)) =
        expression::split_index_lookup(entry_expression)
    else {
        return false;
    };
    let container_key = resolve_state_path(container_expression, state, outputs);
    let Some(current_container) = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let current_entry =
        container_entry(&current_container, &index).unwrap_or_else(|| "None".to_string());
    let Some(updated_entry) = expression::set_option_value(&current_entry, &json_value(value))
    else {
        return false;
    };
    let Some(updated_container) = set_container_entry(&current_container, &index, &updated_entry)
    else {
        return false;
    };
    state.insert(container_key, updated_container);
    true
}

fn apply_container_result_variant_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((entry_expression, variant)) = expression::split_result_variant_projection(target)
    else {
        return false;
    };
    let Some((container_expression, index_expression)) =
        expression::split_index_lookup(entry_expression)
    else {
        return false;
    };
    let container_key = resolve_state_path(container_expression, state, outputs);
    let Some(current_container) = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(current_entry) = container_entry(&current_container, &index) else {
        return false;
    };
    let Some(updated_entry) =
        expression::set_result_variant_value(&current_entry, variant, &json_value(value))
    else {
        return false;
    };
    let Some(updated_container) = set_container_entry(&current_container, &index, &updated_entry)
    else {
        return false;
    };
    state.insert(container_key, updated_container);
    true
}

fn apply_container_option_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((entry_expression, field_path)) =
        expression::split_option_value_field_projection(target)
    else {
        return false;
    };
    let Some((container_expression, index_expression)) =
        expression::split_index_lookup(entry_expression)
    else {
        return false;
    };
    let container_key = resolve_state_path(container_expression, state, outputs);
    let Some(current_container) = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(current_entry) = container_entry(&current_container, &index) else {
        return false;
    };
    let value_projection = format!("{entry_expression}.value");
    let Some(current_object) = expression::resolve_option_value_lookup(&value_projection, |name| {
        (name == entry_expression).then(|| current_entry.clone())
    }) else {
        return false;
    };
    let Some(updated_object) =
        expression::set_object_field_value(&current_object, field_path, &json_value(value))
    else {
        return false;
    };
    let Some(updated_entry) = expression::set_option_value(&current_entry, &updated_object) else {
        return false;
    };
    let Some(updated_container) = set_container_entry(&current_container, &index, &updated_entry)
    else {
        return false;
    };
    state.insert(container_key, updated_container);
    true
}

fn apply_container_result_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((entry_expression, variant, field_path)) =
        expression::split_result_variant_field_projection(target)
    else {
        return false;
    };
    let Some((container_expression, index_expression)) =
        expression::split_index_lookup(entry_expression)
    else {
        return false;
    };
    let container_key = resolve_state_path(container_expression, state, outputs);
    let Some(current_container) = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(current_entry) = container_entry(&current_container, &index) else {
        return false;
    };
    let variant_projection = format!("{entry_expression}.{variant}");
    let Some(current_object) =
        expression::resolve_result_variant_lookup(&variant_projection, |name| {
            (name == entry_expression).then(|| current_entry.clone())
        })
    else {
        return false;
    };
    let Some(updated_object) =
        expression::set_object_field_value(&current_object, field_path, &json_value(value))
    else {
        return false;
    };
    let Some(updated_entry) =
        expression::set_result_variant_value(&current_entry, variant, &updated_object)
    else {
        return false;
    };
    let Some(updated_container) = set_container_entry(&current_container, &index, &updated_entry)
    else {
        return false;
    };
    state.insert(container_key, updated_container);
    true
}

fn apply_option_value_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some(option_expression) = target.trim().strip_suffix(".value").map(str::trim) else {
        return false;
    };
    let option_key = resolve_state_path(option_expression, state, outputs);
    let Some(current_option) = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())
    else {
        return false;
    };
    let Some(updated_option) = expression::set_option_value(&current_option, &json_value(value))
    else {
        return false;
    };
    state.insert(option_key, updated_option);
    true
}

fn apply_result_variant_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((result_expression, variant)) = expression::split_result_variant_projection(target)
    else {
        return false;
    };
    let result_key = resolve_state_path(result_expression, state, outputs);
    let Some(current_result) = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())
    else {
        return false;
    };
    let Some(updated_result) =
        expression::set_result_variant_value(&current_result, variant, &json_value(value))
    else {
        return false;
    };
    state.insert(result_key, updated_result);
    true
}

fn apply_option_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((option_expression, field_path)) =
        expression::split_option_value_field_projection(target)
    else {
        return false;
    };
    let option_key = resolve_state_path(option_expression, state, outputs);
    let Some(current_option) = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())
    else {
        return false;
    };
    let value_projection = format!("{option_expression}.value");
    let Some(current_object) = expression::resolve_option_value_lookup(&value_projection, |name| {
        let key = resolve_state_path(name, state, outputs);
        state
            .get(&key)
            .cloned()
            .or_else(|| outputs.get(&key).cloned())
    }) else {
        return false;
    };
    let Some(updated_object) =
        expression::set_object_field_value(&current_object, field_path, &json_value(value))
    else {
        return false;
    };
    let Some(updated_option) = expression::set_option_value(&current_option, &updated_object)
    else {
        return false;
    };
    state.insert(option_key, updated_option);
    true
}

fn apply_result_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((result_expression, variant, field_path)) =
        expression::split_result_variant_field_projection(target)
    else {
        return false;
    };
    let result_key = resolve_state_path(result_expression, state, outputs);
    let Some(current_result) = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())
    else {
        return false;
    };
    let variant_projection = format!("{result_expression}.{variant}");
    let Some(current_object) =
        expression::resolve_result_variant_lookup(&variant_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })
    else {
        return false;
    };
    let Some(updated_object) =
        expression::set_object_field_value(&current_object, field_path, &json_value(value))
    else {
        return false;
    };
    let Some(updated_result) =
        expression::set_result_variant_value(&current_result, variant, &updated_object)
    else {
        return false;
    };
    state.insert(result_key, updated_result);
    true
}

fn apply_option_container_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((object_expression, field_path)) = expression::split_last_top_level_dot(target) else {
        return false;
    };
    let Some((container_projection, index_expression)) =
        expression::split_index_lookup(object_expression)
    else {
        return false;
    };
    let Some(option_expression) = container_projection
        .trim()
        .strip_suffix(".value")
        .map(str::trim)
    else {
        return false;
    };
    let option_key = resolve_state_path(option_expression, state, outputs);
    let Some(current_option) = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())
    else {
        return false;
    };
    let value_projection = format!("{option_expression}.value");
    let Some(current_container) =
        expression::resolve_option_value_lookup(&value_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let rendered_value = json_value(value);
    let Some(updated_container) =
        set_container_object_field(&current_container, &index, field_path, &rendered_value)
    else {
        return false;
    };
    let Some(updated_option) = expression::set_option_value(&current_option, &updated_container)
    else {
        return false;
    };
    state.insert(option_key, updated_option);
    true
}

fn apply_result_container_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((object_expression, field_path)) = expression::split_last_top_level_dot(target) else {
        return false;
    };
    let Some((container_projection, index_expression)) =
        expression::split_index_lookup(object_expression)
    else {
        return false;
    };
    let Some((result_expression, variant)) =
        expression::split_result_variant_projection(container_projection)
    else {
        return false;
    };
    let result_key = resolve_state_path(result_expression, state, outputs);
    let Some(current_result) = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())
    else {
        return false;
    };
    let Some(current_container) =
        expression::resolve_result_variant_lookup(container_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let rendered_value = json_value(value);
    let Some(updated_container) =
        set_container_object_field(&current_container, &index, field_path, &rendered_value)
    else {
        return false;
    };
    let Some(updated_result) =
        expression::set_result_variant_value(&current_result, variant, &updated_container)
    else {
        return false;
    };
    state.insert(result_key, updated_result);
    true
}

fn apply_option_container_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((container_projection, index_expression)) = expression::split_index_lookup(target)
    else {
        return false;
    };
    let Some(option_expression) = container_projection
        .trim()
        .strip_suffix(".value")
        .map(str::trim)
    else {
        return false;
    };
    let option_key = resolve_state_path(option_expression, state, outputs);
    let Some(current_option) = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())
    else {
        return false;
    };
    let value_projection = format!("{option_expression}.value");
    let Some(current_container) =
        expression::resolve_option_value_lookup(&value_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(updated_container) = set_container_entry(&current_container, &index, value) else {
        return false;
    };
    let Some(updated_option) = expression::set_option_value(&current_option, &updated_container)
    else {
        return false;
    };
    state.insert(option_key, updated_option);
    true
}

fn apply_result_container_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((container_projection, index_expression)) = expression::split_index_lookup(target)
    else {
        return false;
    };
    let Some((result_expression, variant)) =
        expression::split_result_variant_projection(container_projection)
    else {
        return false;
    };
    let result_key = resolve_state_path(result_expression, state, outputs);
    let Some(current_result) = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())
    else {
        return false;
    };
    let Some(current_container) =
        expression::resolve_result_variant_lookup(container_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(updated_container) = set_container_entry(&current_container, &index, value) else {
        return false;
    };
    let Some(updated_result) =
        expression::set_result_variant_value(&current_result, variant, &updated_container)
    else {
        return false;
    };
    state.insert(result_key, updated_result);
    true
}

fn apply_container_option_list_append(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some(entry_expression) = target.trim().strip_suffix(".value").map(str::trim) else {
        return false;
    };
    let Some((container_expression, index_expression)) =
        expression::split_index_lookup(entry_expression)
    else {
        return false;
    };
    let container_key = resolve_state_path(container_expression, state, outputs);
    let Some(current_container) = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let current_entry =
        container_entry(&current_container, &index).unwrap_or_else(|| "None".to_string());
    let current_list = if current_entry.trim() == "None" {
        None
    } else {
        let value_projection = format!("{entry_expression}.value");
        let Some(value) = expression::resolve_option_value_lookup(&value_projection, |name| {
            (name == entry_expression).then(|| current_entry.clone())
        }) else {
            return false;
        };
        Some(value)
    };
    let updated_list = append_list_value(current_list.as_deref(), value);
    let Some(updated_entry) = expression::set_option_value(&current_entry, &updated_list) else {
        return false;
    };
    let Some(updated_container) = set_container_entry(&current_container, &index, &updated_entry)
    else {
        return false;
    };
    state.insert(container_key, updated_container);
    true
}

fn apply_container_result_list_append(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((entry_expression, variant)) = expression::split_result_variant_projection(target)
    else {
        return false;
    };
    let Some((container_expression, index_expression)) =
        expression::split_index_lookup(entry_expression)
    else {
        return false;
    };
    let container_key = resolve_state_path(container_expression, state, outputs);
    let Some(current_container) = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(current_entry) = container_entry(&current_container, &index) else {
        return false;
    };
    let variant_projection = format!("{entry_expression}.{variant}");
    let current_list = expression::resolve_result_variant_lookup(&variant_projection, |name| {
        (name == entry_expression).then(|| current_entry.clone())
    });
    let updated_list = append_list_value(current_list.as_deref(), value);
    let Some(updated_entry) =
        expression::set_result_variant_value(&current_entry, variant, &updated_list)
    else {
        return false;
    };
    let Some(updated_container) = set_container_entry(&current_container, &index, &updated_entry)
    else {
        return false;
    };
    state.insert(container_key, updated_container);
    true
}

fn apply_option_list_append(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some(option_expression) = target.trim().strip_suffix(".value").map(str::trim) else {
        return false;
    };
    let option_key = resolve_state_path(option_expression, state, outputs);
    let Some(current_option) = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())
    else {
        return false;
    };
    let current_list = if current_option.trim() == "None" {
        None
    } else {
        let Some(value) = expression::resolve_option_value_lookup(target, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        }) else {
            return false;
        };
        Some(value)
    };
    let updated_list = append_list_value(current_list.as_deref(), value);
    let Some(updated_option) = expression::set_option_value(&current_option, &updated_list) else {
        return false;
    };
    state.insert(option_key, updated_option);
    true
}

fn apply_result_list_append(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((result_expression, variant)) = expression::split_result_variant_projection(target)
    else {
        return false;
    };
    let result_key = resolve_state_path(result_expression, state, outputs);
    let Some(current_result) = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())
    else {
        return false;
    };
    let current_list = expression::resolve_result_variant_lookup(target, |name| {
        let key = resolve_state_path(name, state, outputs);
        state
            .get(&key)
            .cloned()
            .or_else(|| outputs.get(&key).cloned())
    });
    let updated_list = append_list_value(current_list.as_deref(), value);
    let Some(updated_result) =
        expression::set_result_variant_value(&current_result, variant, &updated_list)
    else {
        return false;
    };
    state.insert(result_key, updated_result);
    true
}

fn apply_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((object_expression, field_name)) = expression::split_last_top_level_dot(target) else {
        return false;
    };
    let Some((container_expression, index_expression)) =
        expression::split_index_lookup(object_expression)
    else {
        return false;
    };
    let container_key = resolve_state_path(container_expression, state, outputs);
    let Some(current_container) = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let rendered_value = json_value(value);
    let Some(updated_container) =
        set_container_object_field(&current_container, &index, field_name, &rendered_value)
    else {
        return false;
    };
    state.insert(container_key, updated_container);
    true
}

fn set_container_object_field(
    container: &str,
    index: &str,
    field_path: &str,
    rendered_value: &str,
) -> Option<String> {
    if let Some(current_object) = expression::list_lookup_value(container, index) {
        let updated_object =
            expression::set_object_field_value(&current_object, field_path, rendered_value)?;
        return expression::set_list_lookup_value(container, index, &updated_object);
    }
    let current_object = expression::map_lookup_value(container, index)?;
    let updated_object =
        expression::set_object_field_value(&current_object, field_path, rendered_value)?;
    expression::set_map_lookup_value(container, index, &updated_object)
}

fn set_container_entry(container: &str, index: &str, value: &str) -> Option<String> {
    if let Some(updated_map) = expression::set_map_lookup_value(container, index, value) {
        return Some(updated_map);
    }
    expression::set_list_lookup_value(container, index, value)
}

fn container_entry(container: &str, index: &str) -> Option<String> {
    if let Some(value) = expression::map_lookup_value(container, index) {
        return Some(value);
    }
    expression::list_lookup_value(container, index)
}

fn remove_container_entry(container: &str, index: &str) -> Option<String> {
    if let Some(updated_map) = expression::remove_map_lookup_value(container, index) {
        return Some(updated_map);
    }
    expression::remove_list_lookup_value(container, index)
}

fn apply_list_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((list_expression, index_expression)) = expression::split_index_lookup(target) else {
        return false;
    };
    let list_key = resolve_state_path(list_expression, state, outputs);
    let Some(current_list) = state
        .get(&list_key)
        .cloned()
        .or_else(|| outputs.get(&list_key).cloned())
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(updated_list) = expression::set_list_lookup_value(&current_list, &index, value) else {
        return false;
    };
    state.insert(list_key, updated_list);
    true
}

fn apply_delete(
    statement: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) {
    if apply_container_option_container_entry_delete(statement.trim(), state, outputs) {
        return;
    }
    if apply_container_result_container_entry_delete(statement.trim(), state, outputs) {
        return;
    }
    if apply_option_container_entry_delete(statement.trim(), state, outputs) {
        return;
    }
    if apply_result_container_entry_delete(statement.trim(), state, outputs) {
        return;
    }
    if apply_container_entry_delete(statement.trim(), state, outputs) {
        return;
    }
    let path = resolve_state_path(statement.trim(), state, outputs);
    let snapshot = state.clone();
    if !delete_collection_path_with(state, &path, |name| {
        outputs
            .get(name)
            .cloned()
            .or_else(|| snapshot.get(name).cloned())
    }) {
        delete_state_path(state, &path);
    }
}

fn apply_container_option_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((container_projection, index_expression)) = expression::split_index_lookup(target)
    else {
        return false;
    };
    let Some(entry_expression) = container_projection
        .trim()
        .strip_suffix(".value")
        .map(str::trim)
    else {
        return false;
    };
    let Some((outer_container_expression, outer_index_expression)) =
        expression::split_index_lookup(entry_expression)
    else {
        return false;
    };
    let outer_container_key = resolve_state_path(outer_container_expression, state, outputs);
    let Some(current_outer_container) = state
        .get(&outer_container_key)
        .cloned()
        .or_else(|| outputs.get(&outer_container_key).cloned())
    else {
        return false;
    };
    let outer_index = evaluate_expression(outer_index_expression, state, outputs);
    let current_entry = container_entry(&current_outer_container, &outer_index)
        .unwrap_or_else(|| "None".to_string());
    if current_entry.trim() == "None" {
        return true;
    }
    let Some(current_inner_container) =
        expression::resolve_option_value_lookup(container_projection, |name| {
            (name == entry_expression).then(|| current_entry.clone())
        })
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(updated_inner_container) = remove_container_entry(&current_inner_container, &index)
    else {
        return false;
    };
    let Some(updated_entry) =
        expression::set_option_value(&current_entry, &updated_inner_container)
    else {
        return false;
    };
    let Some(updated_outer_container) =
        set_container_entry(&current_outer_container, &outer_index, &updated_entry)
    else {
        return false;
    };
    state.insert(outer_container_key, updated_outer_container);
    true
}

fn apply_container_result_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((container_projection, index_expression)) = expression::split_index_lookup(target)
    else {
        return false;
    };
    let Some((entry_expression, variant)) =
        expression::split_result_variant_projection(container_projection)
    else {
        return false;
    };
    let Some((outer_container_expression, outer_index_expression)) =
        expression::split_index_lookup(entry_expression)
    else {
        return false;
    };
    let outer_container_key = resolve_state_path(outer_container_expression, state, outputs);
    let Some(current_outer_container) = state
        .get(&outer_container_key)
        .cloned()
        .or_else(|| outputs.get(&outer_container_key).cloned())
    else {
        return false;
    };
    let outer_index = evaluate_expression(outer_index_expression, state, outputs);
    let Some(current_entry) = container_entry(&current_outer_container, &outer_index) else {
        return false;
    };
    let Some(current_inner_container) =
        expression::resolve_result_variant_lookup(container_projection, |name| {
            (name == entry_expression).then(|| current_entry.clone())
        })
    else {
        return true;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(updated_inner_container) = remove_container_entry(&current_inner_container, &index)
    else {
        return false;
    };
    let Some(updated_entry) =
        expression::set_result_variant_value(&current_entry, variant, &updated_inner_container)
    else {
        return false;
    };
    let Some(updated_outer_container) =
        set_container_entry(&current_outer_container, &outer_index, &updated_entry)
    else {
        return false;
    };
    state.insert(outer_container_key, updated_outer_container);
    true
}

fn apply_option_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((container_projection, index_expression)) = expression::split_index_lookup(target)
    else {
        return false;
    };
    let Some(option_expression) = container_projection
        .trim()
        .strip_suffix(".value")
        .map(str::trim)
    else {
        return false;
    };
    let option_key = resolve_state_path(option_expression, state, outputs);
    let Some(current_option) = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())
    else {
        return false;
    };
    if current_option.trim() == "None" {
        return true;
    }
    let Some(current_container) =
        expression::resolve_option_value_lookup(container_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(updated_container) = remove_container_entry(&current_container, &index) else {
        return false;
    };
    let Some(updated_option) = expression::set_option_value(&current_option, &updated_container)
    else {
        return false;
    };
    state.insert(option_key, updated_option);
    true
}

fn apply_result_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((container_projection, index_expression)) = expression::split_index_lookup(target)
    else {
        return false;
    };
    let Some((result_expression, variant)) =
        expression::split_result_variant_projection(container_projection)
    else {
        return false;
    };
    let result_key = resolve_state_path(result_expression, state, outputs);
    let Some(current_result) = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())
    else {
        return false;
    };
    let Some(current_container) =
        expression::resolve_result_variant_lookup(container_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })
    else {
        return true;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(updated_container) = remove_container_entry(&current_container, &index) else {
        return false;
    };
    let Some(updated_result) =
        expression::set_result_variant_value(&current_result, variant, &updated_container)
    else {
        return false;
    };
    state.insert(result_key, updated_result);
    true
}

fn apply_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let Some((container_expression, index_expression)) = expression::split_index_lookup(target)
    else {
        return false;
    };
    let container_key = resolve_state_path(container_expression, state, outputs);
    let Some(current_value) = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())
    else {
        return false;
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let Some(updated_value) = remove_container_entry(&current_value, &index) else {
        return false;
    };
    state.insert(container_key, updated_value);
    true
}

fn evaluate_predicate(
    predicate: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    predicate::evaluate(predicate, state, outputs)
}

fn has_else_branch(step: &Step) -> bool {
    step.otherwise_call.is_some()
        || step.otherwise_invoke.is_some()
        || !step.otherwise_parallel_invokes.is_empty()
        || !step.otherwise_set_statements.is_empty()
        || !step.otherwise_append_statements.is_empty()
        || !step.otherwise_compute_statements.is_empty()
        || !step.otherwise_delete_statements.is_empty()
}

fn subdocument_for_intent(
    document: &RifDocument,
    target: &str,
) -> Option<crate::rif_model::RifDocument> {
    let intent = document
        .intents
        .iter()
        .find(|intent| intent.name == target)?
        .clone();
    Some(crate::rif_model::RifDocument {
        intent: intent.clone(),
        intents: vec![intent],
        application: document.application.clone(),
        source_path: document.source_path.clone(),
    })
}

fn apply_invocation_bindings(
    invocation: &InvocationTarget,
    document: &RifDocument,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
    child_state: &mut BTreeMap<String, String>,
) {
    let Some(target_intent) = document
        .intents
        .iter()
        .find(|intent| intent.name == invocation.target)
    else {
        return;
    };
    for (name, expression) in &invocation.bindings {
        if target_intent.subjects.contains_key(name) || target_intent.inputs.contains_key(name) {
            if target_intent.subjects.contains_key(name) {
                let source_prefix = resolve_state_path(expression, state, outputs);
                if copy_state_prefix(state, &source_prefix, child_state, name) {
                    continue;
                }
            }
            let value = evaluate_typed_expression(document, expression, state, outputs);
            child_state.insert(name.clone(), value);
        }
    }
}

fn apply_invocation_result_bindings(
    invocation: &InvocationTarget,
    document: &RifDocument,
    parent_state: &BTreeMap<String, String>,
    parent_outputs: &BTreeMap<String, String>,
    child_state: &mut BTreeMap<String, String>,
) {
    let Some(target_intent) = document
        .intents
        .iter()
        .find(|intent| intent.name == invocation.target)
    else {
        return;
    };
    for (target_name, source_expression) in &invocation.bindings {
        if !target_intent.subjects.contains_key(target_name) {
            continue;
        }
        let source_prefix = resolve_state_path(source_expression, parent_state, parent_outputs);
        if target_name == &source_prefix {
            continue;
        }
        copy_state_prefix_from_snapshot(
            child_state.clone(),
            target_name,
            child_state,
            &source_prefix,
        );
        restore_state_prefix(child_state, parent_state, target_name);
    }
}

fn copy_state_prefix(
    source: &BTreeMap<String, String>,
    source_prefix: &str,
    target: &mut BTreeMap<String, String>,
    target_prefix: &str,
) -> bool {
    copy_state_prefix_from_snapshot(source.clone(), source_prefix, target, target_prefix)
}

fn copy_state_prefix_from_snapshot(
    source: BTreeMap<String, String>,
    source_prefix: &str,
    target: &mut BTreeMap<String, String>,
    target_prefix: &str,
) -> bool {
    let mut copied = false;
    let source_dot_prefix = format!("{source_prefix}.");
    let target_dot_prefix = format!("{target_prefix}.");
    for (key, value) in source {
        if key == source_prefix {
            target.insert(target_prefix.to_string(), value);
            copied = true;
        } else if let Some(suffix) = key.strip_prefix(&source_dot_prefix) {
            target.insert(format!("{target_dot_prefix}{suffix}"), value);
            copied = true;
        }
    }
    copied
}

fn restore_state_prefix(
    state: &mut BTreeMap<String, String>,
    original: &BTreeMap<String, String>,
    prefix: &str,
) {
    let dot_prefix = format!("{prefix}.");
    let keys = state
        .keys()
        .filter(|key| *key == prefix || key.starts_with(&dot_prefix))
        .cloned()
        .collect::<Vec<_>>();
    for key in keys {
        state.remove(&key);
    }
    for (key, value) in original {
        if key == prefix || key.starts_with(&dot_prefix) {
            state.insert(key.clone(), value.clone());
        }
    }
}

fn run_parallel_invocations(
    document: &RifDocument,
    targets: &[InvocationTarget],
    base_state: BTreeMap<String, String>,
    base_outputs: BTreeMap<String, String>,
    operation_outputs: BTreeMap<String, String>,
    fail_at: BTreeMap<String, String>,
) -> Result<ParallelJoin, SimulationResult> {
    let mut merged_state = base_state.clone();
    let mut merged_outputs = base_outputs.clone();
    let mut changed_state = BTreeMap::new();
    let mut changed_outputs = BTreeMap::new();
    let mut trace = Vec::new();

    for target in targets {
        trace.push(format!("PARALLEL {}", target.target));
        let Some(subdocument) = subdocument_for_intent(document, &target.target) else {
            return Err(SimulationResult {
                status: "failed".to_string(),
                final_state: merged_state,
                outputs: merged_outputs,
                trace,
                failure: Some("UnknownIntent".to_string()),
            });
        };
        let mut child_state = base_state.clone();
        apply_invocation_bindings(
            target,
            document,
            &base_state,
            &base_outputs,
            &mut child_state,
        );
        let mut result = simulate_with_operation_outputs(
            &subdocument,
            child_state,
            operation_outputs.clone(),
            fail_at.clone(),
        );
        trace.extend(result.trace);
        if result.status == "failed" {
            return Err(SimulationResult {
                status: "failed".to_string(),
                final_state: result.final_state,
                outputs: result.outputs,
                trace,
                failure: result.failure,
            });
        }
        apply_invocation_result_bindings(
            target,
            document,
            &base_state,
            &base_outputs,
            &mut result.final_state,
        );

        for (key, value) in &result.final_state {
            if base_state.get(key) != Some(value)
                && let Some(existing) = changed_state.insert(key.clone(), value.clone())
                && existing != *value
            {
                return Err(SimulationResult {
                    status: "failed".to_string(),
                    final_state: merged_state,
                    outputs: merged_outputs,
                    trace,
                    failure: Some("ParallelJoinConflict".to_string()),
                });
            }
        }
        for (key, value) in &result.outputs {
            if base_outputs.get(key) != Some(value)
                && let Some(existing) = changed_outputs.insert(key.clone(), value.clone())
                && existing != *value
            {
                return Err(SimulationResult {
                    status: "failed".to_string(),
                    final_state: merged_state,
                    outputs: merged_outputs,
                    trace,
                    failure: Some("ParallelJoinConflict".to_string()),
                });
            }
        }
    }

    for (key, value) in changed_state {
        merged_state.insert(key, value);
    }
    for (key, value) in changed_outputs {
        merged_outputs.insert(key, value);
    }

    Ok((merged_state, merged_outputs, trace))
}

fn evaluate_expression(
    expression: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> String {
    expression::evaluate(expression, |token| expression_value(token, state, outputs))
}

fn evaluate_typed_expression(
    document: &crate::rif_model::RifDocument,
    expression: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> String {
    expression::evaluate(expression, |token| {
        typed_object_expression_value(document, token, state, outputs)
            .or_else(|| expression_value(token, state, outputs))
    })
}

fn expression_value(
    token: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    if let Some(value) = collection_path_value_with(state, token, |name| {
        outputs
            .get(name)
            .cloned()
            .or_else(|| state.get(name).cloned())
    }) {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_container_count(token, |name| expression_value(name, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) = expression::resolve_option_value_lookup(token, |name| {
        expression_value(name, state, outputs)
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_result_variant_lookup(token, |name| {
        expression_value(name, state, outputs)
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_object_field_lookup(token, |name| {
        expression_value(name, state, outputs)
    }) {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_map_lookup(token, |name| expression_value(name, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_list_lookup(token, |name| expression_value(name, state, outputs))
    {
        return Some(value);
    }
    let token = resolve_state_path(token, state, outputs);
    outputs
        .get(&token)
        .cloned()
        .or_else(|| state.get(&token).cloned())
}
fn resolve_state_path(
    token: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> String {
    if !token.contains('[') {
        return token.to_string();
    }

    let mut resolved = String::new();
    let mut rest = token;
    while let Some(open) = rest.find('[') {
        resolved.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            return token.to_string();
        };
        let inner = &after_open[..close];
        if inner.contains('=') {
            resolved.push('[');
            resolved.push_str(inner);
            resolved.push(']');
            rest = &after_open[close + 1..];
            continue;
        }
        let value = evaluate_expression(inner.trim(), state, outputs);
        if !resolved.is_empty() && !resolved.ends_with('.') {
            resolved.push('.');
        }
        resolved.push_str(&value);
        rest = &after_open[close + 1..];
    }
    resolved.push_str(rest);
    resolved
}

fn delete_state_path(state: &mut BTreeMap<String, String>, path: &str) {
    let prefix = format!("{path}.");
    state.retain(|key, _| key != path && !key.starts_with(&prefix));
}

fn tokens(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_alphabetic())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}
