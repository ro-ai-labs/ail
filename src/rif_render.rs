use crate::rif_model::{
    Application, EndpointDefinition, Intent, InvocationTarget, OutputValue, RifDocument, Step,
    ThingDefinition, TriggerDefinition,
};

pub fn render_rif_document(document: &RifDocument) -> String {
    let mut lines = Vec::new();
    let mut wrote_application = false;

    if let Some(name) = document.application.name.as_ref() {
        lines.push(format!("app {name}"));
        wrote_application = true;
    }

    if let Some(module) = document.application.module.as_ref() {
        if wrote_application {
            lines.push(String::new());
        }
        lines.push(format!("module {module}"));
        wrote_application = true;
    }

    if render_application(&document.application, &mut lines) {
        wrote_application = true;
    }

    let intents: Vec<&Intent> = if document.intents.is_empty() {
        vec![&document.intent]
    } else {
        document.intents.iter().collect()
    };

    for (index, intent) in intents.into_iter().enumerate() {
        if wrote_application || index > 0 {
            lines.push(String::new());
        }
        render_intent(intent, &mut lines);
    }

    lines.join("\n")
}

fn render_application(application: &Application, lines: &mut Vec<String>) -> bool {
    let mut wrote_section = false;

    if !application.exports.is_empty() {
        push_blank_line(lines);
        lines.push("exports:".to_string());
        for export in &application.exports {
            lines.push(format!("  export {} {}", export.kind, export.name));
        }
        wrote_section = true;
    }

    if !application.enums.is_empty() {
        push_blank_line(lines);
        lines.push("types:".to_string());
        for enum_definition in application.enums.values() {
            lines.push(format!("  enum {}", enum_definition.name));
            for value in &enum_definition.values {
                lines.push(format!("    value {value}"));
            }
        }
        wrote_section = true;
    }

    if !application.things.is_empty() {
        push_blank_line(lines);
        lines.push("things:".to_string());
        for thing in application.things.values() {
            render_thing(thing, lines);
        }
        wrote_section = true;
    }

    if !application.collections.is_empty() {
        push_blank_line(lines);
        lines.push("collections:".to_string());
        for collection in application.collections.values() {
            lines.push(format!(
                "  collection {}: {}",
                collection.name, collection.type_name
            ));
            if !collection.unique_fields.is_empty() {
                lines.push(format!(
                    "    unique: {}",
                    collection.unique_fields.join(", ")
                ));
            }
        }
        wrote_section = true;
    }

    if !application.operations.is_empty() {
        push_blank_line(lines);
        lines.push("operations:".to_string());
        for operation in application.operations.values() {
            lines.push(format!("  {}", render_operation_signature(operation)));
            if operation_outputs_render_as_block(operation) {
                for output in &operation.outputs {
                    lines.push(format!("    output: {}: {}", output.name, output.type_name));
                }
            }
            for target in &operation.reads {
                lines.push(format!("    reads: {target}"));
            }
            for target in &operation.changes {
                lines.push(format!("    changes: {target}"));
            }
            for target in &operation.external_calls {
                lines.push(format!("    external call: {target}"));
            }
            for target in &operation.may_fail {
                lines.push(format!("    may fail with: {target}"));
            }
        }
        wrote_section = true;
    }

    if !application.endpoints.is_empty() {
        push_blank_line(lines);
        lines.push("endpoints:".to_string());
        for endpoint in &application.endpoints {
            render_endpoint(endpoint, lines);
        }
        wrote_section = true;
    }

    if !application.triggers.is_empty() {
        push_blank_line(lines);
        lines.push("triggers:".to_string());
        for trigger in &application.triggers {
            render_trigger(trigger, lines);
        }
        wrote_section = true;
    }

    wrote_section
}

fn render_thing(thing: &ThingDefinition, lines: &mut Vec<String>) {
    lines.push(format!("  thing {}", thing.name));
    for field in thing.fields.values() {
        lines.push(format!("    field {}: {}", field.name, field.type_name));
    }
}

fn render_operation_signature(operation: &crate::rif_model::OperationDefinition) -> String {
    let args = if operation.input_order.is_empty() {
        operation
            .inputs
            .iter()
            .map(|(name, type_name)| format!("{name}: {type_name}"))
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        operation
            .input_order
            .iter()
            .filter_map(|name| {
                operation
                    .inputs
                    .get(name)
                    .map(|type_name| format!("{name}: {type_name}"))
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut signature = format!("operation {}({args})", operation.name);
    if let Some(output) = operation.outputs.first()
        && !output.type_name.is_empty()
        && output.type_name != "Unit"
        && output.name == "result"
    {
        signature.push_str(&format!(" -> {}", output.type_name));
    }
    signature
}

fn operation_outputs_render_as_block(operation: &crate::rif_model::OperationDefinition) -> bool {
    !(operation.outputs.is_empty()
        || operation.outputs.len() == 1 && operation.outputs[0].name == "result")
}

fn render_endpoint(endpoint: &EndpointDefinition, lines: &mut Vec<String>) {
    lines.push(format!(
        "  endpoint {} {} -> {}",
        endpoint.method, endpoint.path, endpoint.target
    ));
    if !endpoint.request_fields.is_empty() {
        lines.push("    request:".to_string());
        for (name, type_name) in &endpoint.request_fields {
            lines.push(format!("      {name}: {type_name}"));
        }
    }
    if !endpoint.requires.is_empty() {
        lines.push("    requires:".to_string());
        for requirement in &endpoint.requires {
            lines.push(format!("      {requirement}"));
        }
    }
    if !endpoint.bindings.is_empty() {
        lines.push("    bind:".to_string());
        for (target, source) in &endpoint.bindings {
            lines.push(format!("      {target} = {source}"));
        }
    }
    if endpoint.response_status.is_some()
        || !endpoint.response_fields.is_empty()
        || !endpoint.responses.is_empty()
    {
        lines.push("    respond:".to_string());
        for (name, type_name) in &endpoint.response_fields {
            lines.push(format!("      {name}: {type_name}"));
        }
        if let Some(status) = endpoint.response_status.as_ref() {
            lines.push(format!("      status: {status}"));
        }
        for (name, source) in &endpoint.responses {
            lines.push(format!("      {name} = {source}"));
        }
    }
    if endpoint.error_status.is_some()
        || !endpoint.error_fields.is_empty()
        || !endpoint.error_responses.is_empty()
    {
        lines.push("    error:".to_string());
        if let Some(status) = endpoint.error_status.as_ref() {
            lines.push(format!("      status: {status}"));
        }
        for (name, type_name) in &endpoint.error_fields {
            lines.push(format!("      {name}: {type_name}"));
        }
        for (name, source) in &endpoint.error_responses {
            lines.push(format!("      {name} = {source}"));
        }
    }
    for (name, error) in &endpoint.error_cases {
        lines.push(format!("    error {name}:"));
        if let Some(status) = error.status.as_ref() {
            lines.push(format!("      status: {status}"));
        }
        for (field, type_name) in &error.response_fields {
            lines.push(format!("      {field}: {type_name}"));
        }
        for (field, source) in &error.responses {
            lines.push(format!("      {field} = {source}"));
        }
    }
}

fn render_trigger(trigger: &TriggerDefinition, lines: &mut Vec<String>) {
    lines.push(format!("  trigger {} -> {}", trigger.name, trigger.target));
    if let Some(schedule) = trigger.schedule.as_ref() {
        lines.push(format!("    schedule: {schedule}"));
    }
    if let Some(queue) = trigger.queue.as_ref() {
        lines.push(format!("    queue: {queue}"));
    }
    if !trigger.payload_fields.is_empty() {
        lines.push("    payload:".to_string());
        for (name, type_name) in &trigger.payload_fields {
            lines.push(format!("      {name}: {type_name}"));
        }
    }
    if !trigger.requires.is_empty() {
        lines.push("    requires:".to_string());
        for requirement in &trigger.requires {
            lines.push(format!("      {requirement}"));
        }
    }
    if !trigger.bindings.is_empty() {
        lines.push("    bind:".to_string());
        for (target, source) in &trigger.bindings {
            lines.push(format!("      {target} = {source}"));
        }
    }
}

fn render_intent(intent: &Intent, lines: &mut Vec<String>) {
    lines.push(format!("intent {}", intent.name));

    if !intent.subjects.is_empty() {
        lines.push(String::new());
        lines.push("subject:".to_string());
        for thing in intent.subjects.values() {
            lines.push(format!("  {}: {}", thing.name, thing.type_name));
        }
    }

    if !intent.inputs.is_empty() {
        lines.push(String::new());
        lines.push("inputs:".to_string());
        for thing in intent.inputs.values() {
            lines.push(format!("  {}: {}", thing.name, thing.type_name));
        }
    }

    if !intent.requires.is_empty() {
        lines.push(String::new());
        lines.push("requires:".to_string());
        for requirement in &intent.requires {
            lines.push(format!("  {}", requirement.text));
        }
    }

    if !intent.state_transitions.is_empty() {
        lines.push(String::new());
        lines.push("state transition:".to_string());
        for transition in &intent.state_transitions {
            lines.push(format!(
                "  {}: {} -> {}",
                transition.field_path, transition.from_state, transition.to_state
            ));
        }
    }

    if intent.step_schedule != "sequential" {
        lines.push(String::new());
        lines.push("steps:".to_string());
        lines.push(format!("  schedule: {}", intent.step_schedule));
        render_steps(&intent.steps, lines);
    } else if !intent.steps.is_empty() {
        lines.push(String::new());
        lines.push("steps:".to_string());
        render_steps(&intent.steps, lines);
    }

    if !intent.failure_handlers.is_empty() {
        lines.push(String::new());
        lines.push("failure behavior:".to_string());
        for handler in &intent.failure_handlers {
            lines.push(format!("  if {}:", handler.condition));
            let has_stop_line = handler
                .actions
                .iter()
                .any(|action| action.starts_with("stop with "));
            for action in &handler.actions {
                lines.push(format!("    {action}"));
            }
            if let Some(stop_failure) = handler.stop_failure.as_ref()
                && !has_stop_line
            {
                lines.push(format!("    stop with {stop_failure}"));
            }
        }
    }

    if !intent.guarantees.is_empty() {
        lines.push(String::new());
        lines.push("guarantees:".to_string());
        for guarantee in &intent.guarantees {
            let condition =
                if guarantee.conditions.len() == 1 && guarantee.conditions[0].starts_with("if ") {
                    guarantee.conditions[0].clone()
                } else if guarantee.conditions.is_empty() {
                    "if this intent succeeds".to_string()
                } else {
                    format!("if {}", guarantee.conditions.join(" and "))
                };
            lines.push(format!("  {condition}:"));
            for statement in &guarantee.statements {
                lines.push(format!("    {statement}"));
            }
        }
    }

    if !intent.unresolved_questions.is_empty() {
        lines.push(String::new());
        lines.push("unresolved questions:".to_string());
        for question in &intent.unresolved_questions {
            let mut lines_iter = question.text.lines();
            if let Some(first_line) = lines_iter.next() {
                lines.push(format!("  - {first_line}"));
                for continuation in lines_iter {
                    lines.push(format!("      {continuation}"));
                }
            }
        }
    }

    if !intent.returns.is_empty() {
        lines.push(String::new());
        lines.push("returns:".to_string());
        for return_value in &intent.returns {
            lines.push(format!("  {}: {}", return_value.name, return_value.source));
        }
    }
}

fn render_steps(steps: &[Step], lines: &mut Vec<String>) {
    for step in steps {
        lines.push(format!("  {}. {}", step.number, step.title));
        if let Some(guard) = step.guard.as_ref() {
            lines.push(format!("     when: {guard}"));
        }
        if let Some(condition) = step.repeat_while.as_ref() {
            lines.push(format!("     repeat while: {condition}"));
        }
        if let Some(condition) = step.repeat_until.as_ref() {
            lines.push(format!("     repeat until: {condition}"));
        }
        if let Some(call) = step.call.as_ref() {
            lines.push(format!("     call: {}", call.expression));
        }
        if let Some(call) = step.otherwise_call.as_ref() {
            lines.push(format!("     otherwise call: {}", call.expression));
        }
        if let Some(invoke) = step.invoke.as_ref() {
            lines.push(format!("     invoke: {}", render_invocation(invoke)));
        }
        if let Some(invoke) = step.otherwise_invoke.as_ref() {
            lines.push(format!(
                "     otherwise invoke: {}",
                render_invocation(invoke)
            ));
        }
        if !step.parallel_invokes.is_empty() {
            lines.push(format!(
                "     parallel invoke: {}",
                render_invocation_list(&step.parallel_invokes)
            ));
        }
        if !step.otherwise_parallel_invokes.is_empty() {
            lines.push(format!(
                "     otherwise parallel invoke: {}",
                render_invocation_list(&step.otherwise_parallel_invokes)
            ));
        }
        for statement in &step.set_statements {
            lines.push(format!("     set: {statement}"));
        }
        for statement in &step.otherwise_set_statements {
            lines.push(format!("     otherwise set: {statement}"));
        }
        for statement in &step.append_statements {
            lines.push(format!("     append: {statement}"));
        }
        for statement in &step.otherwise_append_statements {
            lines.push(format!("     otherwise append: {statement}"));
        }
        for statement in &step.compute_statements {
            lines.push(format!("     compute: {statement}"));
        }
        for statement in &step.otherwise_compute_statements {
            lines.push(format!("     otherwise compute: {statement}"));
        }
        for statement in &step.delete_statements {
            lines.push(format!("     delete: {statement}"));
        }
        for statement in &step.otherwise_delete_statements {
            lines.push(format!("     otherwise delete: {statement}"));
        }
        if let Some(iterate_over) = step.iterate_over.as_ref() {
            let item = step.iteration_item.as_deref().unwrap_or("item");
            lines.push(format!("     for each: {iterate_over} as {item}"));
        }
        for output in step.outputs.values() {
            lines.push(format!("     output: {}", render_output_value(output)));
        }
        for read in &step.reads {
            lines.push(format!("     reads: {read}"));
        }
        for change in &step.changes {
            lines.push(format!("     changes: {change}"));
        }
        for call in &step.external_calls {
            lines.push(format!("     external call: {call}"));
        }
        for failure in &step.may_fail {
            lines.push(format!("     may fail with: {failure}"));
        }
        if let Some(compensation) = step.compensation.as_ref() {
            lines.push(format!("     compensation: {compensation}"));
        }
        for failure in &step.ignored_failures {
            lines.push(format!("     ignore failure: {failure}"));
        }
        for raw in &step.raw_lines {
            lines.push(format!("     {raw}"));
        }
    }
}

fn render_invocation(invocation: &InvocationTarget) -> String {
    if invocation.bindings.is_empty() {
        return invocation.target.clone();
    }
    let bindings = invocation
        .bindings
        .iter()
        .map(|(name, value)| format!("{name} = {value}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({bindings})", invocation.target)
}

fn render_invocation_list(invocations: &[InvocationTarget]) -> String {
    invocations
        .iter()
        .map(render_invocation)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_output_value(output: &OutputValue) -> String {
    if output.type_name.is_empty() {
        output.name.clone()
    } else {
        format!("{}: {}", output.name, output.type_name)
    }
}

fn push_blank_line(lines: &mut Vec<String>) {
    if !lines.is_empty() && !lines.last().is_some_and(String::is_empty) {
        lines.push(String::new());
    }
}
