use std::collections::BTreeMap;

use crate::core_model::{
    CoreEffect, CoreModule, CoreOperation, Graph, Node, PermissionRequirement, Program, attr,
};
use crate::expression;
use crate::rif_model::{
    Application, CollectionDefinition, EndpointDefinition, Intent, RifDocument, Step,
    TriggerDefinition,
};

pub fn build_program(document: &RifDocument) -> Program {
    let mut graph = Graph::default();
    let mut permissions: BTreeMap<(String, String), PermissionRequirement> = BTreeMap::new();
    let mut effects: BTreeMap<(String, String), CoreEffect> = BTreeMap::new();
    let mut operations: BTreeMap<String, CoreOperation> = BTreeMap::new();
    let module_name = document
        .application
        .module
        .clone()
        .or_else(|| document.application.name.clone())
        .unwrap_or_else(|| "main".to_string());
    let module_node = graph.add_node("module", &module_name, None, BTreeMap::new());

    for thing in document.application.things.values() {
        let thing_node = graph.add_node(
            "thing",
            &thing.name,
            Some(thing.name.clone()),
            BTreeMap::new(),
        );
        graph.add_edge("contains", &module_node, &thing_node, BTreeMap::new());
        for field in thing.fields.values() {
            let field_name = format!("{}.{}", thing.name, field.name);
            let field_node = graph.add_node(
                "field",
                field_name,
                Some(field.type_name.clone()),
                attr(&[("secret", if field.is_secret { "true" } else { "false" })]),
            );
            graph.add_edge("has_field", &thing_node, &field_node, BTreeMap::new());
            for state in state_type_values(&field.type_name) {
                let state_node = graph.add_node("state", state, None, BTreeMap::new());
                graph.add_edge("has_state", &field_node, &state_node, BTreeMap::new());
            }
        }
    }

    for collection in document.application.collections.values() {
        let collection_node = build_collection(&mut graph, collection);
        graph.add_edge("contains", &module_node, &collection_node, BTreeMap::new());
    }

    for operation in document.application.operations.values() {
        let operation_node = graph.add_node(
            "operation",
            &operation.name,
            None,
            attr(&[("declared", "true")]),
        );
        graph.add_edge("contains", &module_node, &operation_node, BTreeMap::new());
        operations.insert(
            operation.name.clone(),
            CoreOperation {
                name: operation.name.clone(),
                inputs: operation.inputs.values().cloned().collect(),
                outputs: operation
                    .outputs
                    .iter()
                    .map(|output| output.type_name.clone())
                    .collect(),
            },
        );
    }

    for endpoint in &document.application.endpoints {
        let endpoint_node = build_endpoint(&mut graph, &document.application, endpoint);
        graph.add_edge("contains", &module_node, &endpoint_node, BTreeMap::new());
    }

    for trigger in &document.application.triggers {
        let trigger_node = build_trigger(&mut graph, &document.application, trigger);
        graph.add_edge("contains", &module_node, &trigger_node, BTreeMap::new());
    }

    for intent in &document.intents {
        let intent_node = build_intent(
            &mut graph,
            &mut operations,
            &mut permissions,
            &mut effects,
            &document.application,
            intent,
        );
        graph.add_edge("contains", &module_node, &intent_node, BTreeMap::new());
    }

    let mut metadata = BTreeMap::new();
    metadata.insert("intent".to_string(), document.intent.name.clone());
    if let Some(path) = &document.source_path {
        metadata.insert("source_path".to_string(), path.clone());
    }

    Program {
        modules: vec![CoreModule {
            name: module_name,
            intents: document
                .intents
                .iter()
                .map(|intent| intent.name.clone())
                .collect(),
        }],
        graph,
        operations: operations.into_values().collect(),
        permissions: permissions.into_values().collect(),
        effects: effects.into_values().collect(),
        views: Vec::new(),
        metadata,
    }
}

fn build_collection(graph: &mut Graph, collection: &CollectionDefinition) -> Node {
    let collection_node = graph.add_node(
        "collection",
        &collection.name,
        Some(collection.type_name.clone()),
        BTreeMap::new(),
    );
    let type_node = graph.add_node(
        "thing",
        &collection.type_name,
        Some(collection.type_name.clone()),
        BTreeMap::new(),
    );
    graph.add_edge("stores", &collection_node, &type_node, BTreeMap::new());
    collection_node
}

pub fn infer_permissions(document: &RifDocument) -> Vec<PermissionRequirement> {
    build_program(document).permissions
}

fn build_intent(
    graph: &mut Graph,
    operations: &mut BTreeMap<String, CoreOperation>,
    permissions: &mut BTreeMap<(String, String), PermissionRequirement>,
    effects: &mut BTreeMap<(String, String), CoreEffect>,
    application: &Application,
    intent: &Intent,
) -> Node {
    let intent_node = graph.add_node(
        "intent",
        &intent.name,
        None,
        attr(&[("schedule", &intent.step_schedule)]),
    );

    for (name, thing) in &intent.subjects {
        let node = graph.add_node(
            "thing",
            name,
            Some(thing.type_name.clone()),
            attr(&[("secret", if thing.is_secret { "true" } else { "false" })]),
        );
        graph.add_edge("contains", &intent_node, &node, BTreeMap::new());
    }

    for (name, thing) in &intent.inputs {
        let node = graph.add_node(
            "value",
            name,
            Some(thing.type_name.clone()),
            attr(&[("secret", if thing.is_secret { "true" } else { "false" })]),
        );
        graph.add_edge("contains", &intent_node, &node, BTreeMap::new());
    }

    for requirement in &intent.requires {
        let requirement_node = graph.add_node(
            "contract",
            &requirement.text,
            None,
            attr(&[("kind", "requirement")]),
        );
        graph.add_edge("requires", &intent_node, &requirement_node, BTreeMap::new());
        if let Some(target) = first_reference(&requirement.text) {
            add_permission(graph, permissions, "Read", &target, &intent_node);
        }
    }

    for transition in &intent.state_transitions {
        let field_node = graph.add_node("field", &transition.field_path, None, BTreeMap::new());
        if let Some((owner, _)) = transition.field_path.split_once('.')
            && let Some(owner_node) = graph
                .find_node("thing", owner)
                .or_else(|| graph.find_node("value", owner))
                .cloned()
        {
            graph.add_edge("has_field", &owner_node, &field_node, BTreeMap::new());
        }
        let from_state = graph.add_node("state", &transition.from_state, None, BTreeMap::new());
        let to_state = graph.add_node("state", &transition.to_state, None, BTreeMap::new());
        graph.add_edge("has_state", &field_node, &from_state, BTreeMap::new());
        graph.add_edge("has_state", &field_node, &to_state, BTreeMap::new());
        graph.add_edge(
            "transitions_to",
            &from_state,
            &to_state,
            attr(&[("field", &transition.field_path)]),
        );
    }

    let mut previous_step = None;
    for step in &intent.steps {
        let step_node = graph.add_node(
            "step",
            &step.title,
            None,
            attr(&[("number", &step.number.to_string())]),
        );
        graph.add_edge("contains", &intent_node, &step_node, BTreeMap::new());
        if intent.step_schedule == "sequential"
            && let Some(previous) = &previous_step
        {
            graph.add_edge("order", previous, &step_node, BTreeMap::new());
        }
        previous_step = Some(step_node.clone());

        build_step(
            graph,
            operations,
            permissions,
            effects,
            application,
            step,
            &step_node,
        );
    }

    for handler in &intent.failure_handlers {
        let handler_node = graph.add_node("failure", &handler.condition, None, BTreeMap::new());
        graph.add_edge(
            "handles_failure",
            &intent_node,
            &handler_node,
            BTreeMap::new(),
        );
        if let Some(stop_failure) = &handler.stop_failure {
            let stop_node = graph.add_node("failure", stop_failure, None, BTreeMap::new());
            graph.add_edge("produces", &handler_node, &stop_node, BTreeMap::new());
        }
    }

    for guarantee in &intent.guarantees {
        for statement in &guarantee.statements {
            let guarantee_node = graph.add_node("guarantee", statement, None, BTreeMap::new());
            graph.add_edge("ensures", &intent_node, &guarantee_node, BTreeMap::new());
        }
    }
    intent_node
}

fn build_endpoint(
    graph: &mut Graph,
    application: &Application,
    endpoint: &EndpointDefinition,
) -> Node {
    let endpoint_name = format!("{} {}", endpoint.method, endpoint.path);
    let endpoint_node = graph.add_node(
        "endpoint",
        &endpoint_name,
        None,
        attr(&[("method", &endpoint.method), ("path", &endpoint.path)]),
    );
    let intent_node = graph.add_node("intent", &endpoint.target, None, BTreeMap::new());
    graph.add_edge("routes_to", &endpoint_node, &intent_node, BTreeMap::new());
    if let Some(intent) = application
        .endpoints
        .iter()
        .find(|candidate| candidate.method == endpoint.method && candidate.path == endpoint.path)
    {
        for (target, source) in &intent.bindings {
            let binding_node = graph.add_node(
                "binding",
                format!("{target}={source}"),
                None,
                BTreeMap::new(),
            );
            graph.add_edge("binds", &endpoint_node, &binding_node, BTreeMap::new());
        }
    }
    endpoint_node
}

fn build_trigger(
    graph: &mut Graph,
    application: &Application,
    trigger: &TriggerDefinition,
) -> Node {
    let mut attrs = vec![("name", trigger.name.as_str())];
    if let Some(schedule) = &trigger.schedule {
        attrs.push(("schedule", schedule.as_str()));
    }
    if let Some(queue) = &trigger.queue {
        attrs.push(("queue", queue.as_str()));
    }
    let trigger_node = graph.add_node("trigger", &trigger.name, None, attr(&attrs));
    let intent_node = graph.add_node("intent", &trigger.target, None, BTreeMap::new());
    graph.add_edge("triggers", &trigger_node, &intent_node, BTreeMap::new());
    for requirement in &trigger.requires {
        let requirement_node = graph.add_node(
            "contract",
            requirement,
            None,
            attr(&[("kind", "requirement")]),
        );
        graph.add_edge(
            "requires",
            &trigger_node,
            &requirement_node,
            BTreeMap::new(),
        );
    }
    for (target, source) in &trigger.bindings {
        let binding_node = graph.add_node(
            "binding",
            format!("{target}={source}"),
            None,
            BTreeMap::new(),
        );
        graph.add_edge("binds", &trigger_node, &binding_node, BTreeMap::new());
    }
    let _ = application;
    trigger_node
}

fn build_step(
    graph: &mut Graph,
    operations: &mut BTreeMap<String, CoreOperation>,
    permissions: &mut BTreeMap<(String, String), PermissionRequirement>,
    effects: &mut BTreeMap<(String, String), CoreEffect>,
    application: &Application,
    step: &Step,
    step_node: &crate::core_model::Node,
) {
    let operation_contract = step
        .call
        .as_ref()
        .and_then(|call| application.operations.get(&call.target));

    if let Some(guard) = &step.guard {
        let guard_node = graph.add_node("contract", guard, None, attr(&[("kind", "guard")]));
        graph.add_edge("requires", step_node, &guard_node, BTreeMap::new());
        if let Some(target) = first_reference(guard) {
            add_permission(graph, permissions, "Read", &target, step_node);
        }
    }

    if let Some(call) = &step.call {
        let operation_node = graph.add_node(
            "operation",
            &call.target,
            None,
            attr(&[("expression", &call.expression)]),
        );
        graph.add_edge("calls", step_node, &operation_node, BTreeMap::new());
        operations
            .entry(call.target.clone())
            .or_insert(CoreOperation {
                name: call.target.clone(),
                inputs: call.args.clone(),
                outputs: step.outputs.keys().cloned().collect(),
            });
        add_effect(graph, effects, "call", &call.target, step_node);
        for arg in &call.args {
            add_permission(graph, permissions, "Read", arg, step_node);
        }
    }

    if let Some(target) = &step.invoke {
        let intent_node = graph.add_node("intent", &target.target, None, BTreeMap::new());
        graph.add_edge("invokes", step_node, &intent_node, BTreeMap::new());
    }

    for target in &step.parallel_invokes {
        let intent_node = graph.add_node("intent", &target.target, None, BTreeMap::new());
        graph.add_edge("invokes", step_node, &intent_node, BTreeMap::new());
    }

    if let Some(call) = &step.otherwise_call {
        let operation_node = graph.add_node(
            "operation",
            &call.target,
            None,
            attr(&[("expression", &call.expression)]),
        );
        graph.add_edge("calls", step_node, &operation_node, BTreeMap::new());
        operations
            .entry(call.target.clone())
            .or_insert(CoreOperation {
                name: call.target.clone(),
                inputs: call.args.clone(),
                outputs: step.outputs.keys().cloned().collect(),
            });
        add_effect(graph, effects, "call", &call.target, step_node);
        for arg in &call.args {
            add_permission(graph, permissions, "Read", arg, step_node);
        }
    }

    if let Some(target) = &step.otherwise_invoke {
        let intent_node = graph.add_node("intent", &target.target, None, BTreeMap::new());
        graph.add_edge("invokes", step_node, &intent_node, BTreeMap::new());
    }

    for target in &step.otherwise_parallel_invokes {
        let intent_node = graph.add_node("intent", &target.target, None, BTreeMap::new());
        graph.add_edge("invokes", step_node, &intent_node, BTreeMap::new());
    }

    if let Some(operation) = operation_contract {
        for target in &operation.reads {
            add_permission(graph, permissions, "Read", target, step_node);
        }
        for target in &operation.changes {
            add_permission(graph, permissions, "Change", target, step_node);
            add_effect(graph, effects, "write", target, step_node);
            let target_node = graph.add_node("value", target, None, BTreeMap::new());
            graph.add_edge("changes", step_node, &target_node, BTreeMap::new());
        }
        for service in &operation.external_calls {
            let effect_node = graph.add_node(
                "effect",
                format!("call {service}"),
                None,
                attr(&[("kind", "call"), ("target", service)]),
            );
            graph.add_edge("calls", step_node, &effect_node, BTreeMap::new());
            add_effect(graph, effects, "call", service, step_node);
        }
        for failure in &operation.may_fail {
            let failure_node = graph.add_node("failure", failure, None, BTreeMap::new());
            graph.add_edge("may_fail", step_node, &failure_node, BTreeMap::new());
        }
    }

    for statement in step
        .set_statements
        .iter()
        .chain(step.otherwise_set_statements.iter())
    {
        if let Some((left, right)) = split_set(statement) {
            add_permission(graph, permissions, "Change", &left, step_node);
            add_effect(graph, effects, "write", &left, step_node);
            if looks_like_reference(&right) {
                add_permission(graph, permissions, "Read", &right, step_node);
            }
        }
    }

    for statement in step
        .compute_statements
        .iter()
        .chain(step.otherwise_compute_statements.iter())
    {
        if let Some((left, expression)) = split_set(statement) {
            add_permission(graph, permissions, "Change", &left, step_node);
            add_effect(graph, effects, "write", &left, step_node);
            for reference in expression::references(&expression) {
                add_permission(graph, permissions, "Read", &reference, step_node);
            }
        }
    }

    for statement in step
        .append_statements
        .iter()
        .chain(step.otherwise_append_statements.iter())
    {
        if let Some((left, right)) = split_append(statement) {
            add_permission(graph, permissions, "Change", &left, step_node);
            add_effect(graph, effects, "append", &left, step_node);
            if looks_like_reference(&right) {
                add_permission(graph, permissions, "Read", &right, step_node);
            }
        }
    }

    for statement in step
        .delete_statements
        .iter()
        .chain(step.otherwise_delete_statements.iter())
    {
        add_permission(graph, permissions, "Change", statement, step_node);
        add_effect(graph, effects, "delete", statement, step_node);
    }

    for target in &step.reads {
        add_permission(graph, permissions, "Read", target, step_node);
    }

    for target in &step.changes {
        add_permission(graph, permissions, "Change", target, step_node);
        add_effect(graph, effects, "write", target, step_node);
        let target_node = graph.add_node("value", target, None, BTreeMap::new());
        graph.add_edge("changes", step_node, &target_node, BTreeMap::new());
    }

    for service in &step.external_calls {
        let effect_node = graph.add_node(
            "effect",
            format!("call {service}"),
            None,
            attr(&[("kind", "call"), ("target", service)]),
        );
        graph.add_edge("calls", step_node, &effect_node, BTreeMap::new());
        add_effect(graph, effects, "call", service, step_node);
    }

    for output in step.outputs.values() {
        let value_node = graph.add_node(
            "value",
            &output.name,
            Some(output.type_name.clone()),
            attr(&[("secret", if output.is_secret { "true" } else { "false" })]),
        );
        graph.add_edge("produces", step_node, &value_node, BTreeMap::new());
    }

    for failure in &step.may_fail {
        let failure_node = graph.add_node("failure", failure, None, BTreeMap::new());
        graph.add_edge("may_fail", step_node, &failure_node, BTreeMap::new());
    }

    if let Some(compensation) = &step.compensation {
        let compensation_node = graph.add_node(
            "operation",
            compensation,
            None,
            attr(&[("kind", "compensation")]),
        );
        graph.add_edge(
            "compensates",
            step_node,
            &compensation_node,
            BTreeMap::new(),
        );
    }
}

fn add_permission(
    graph: &mut Graph,
    permissions: &mut BTreeMap<(String, String), PermissionRequirement>,
    kind: &str,
    target: &str,
    source_node: &crate::core_model::Node,
) {
    if target.is_empty() {
        return;
    }
    permissions
        .entry((kind.to_string(), target.to_string()))
        .or_insert(PermissionRequirement {
            kind: kind.to_string(),
            target: target.to_string(),
        });
    let permission_name = format!("{kind} {target}");
    let permission_node = graph.add_node(
        "permission",
        permission_name,
        None,
        attr(&[("kind", kind), ("target", target)]),
    );
    let target_node = graph.add_node("value", target, None, BTreeMap::new());
    graph.add_edge(
        if kind == "Read" { "reads" } else { "changes" },
        source_node,
        &target_node,
        BTreeMap::new(),
    );
    graph.add_edge("requires", source_node, &permission_node, BTreeMap::new());
}

fn add_effect(
    graph: &mut Graph,
    effects: &mut BTreeMap<(String, String), CoreEffect>,
    kind: &str,
    target: &str,
    source_node: &crate::core_model::Node,
) {
    effects
        .entry((kind.to_string(), target.to_string()))
        .or_insert(CoreEffect {
            kind: kind.to_string(),
            target: target.to_string(),
        });
    let effect_node = graph.add_node(
        "effect",
        format!("{kind} {target}"),
        None,
        attr(&[("kind", kind), ("target", target)]),
    );
    graph.add_edge("requires", source_node, &effect_node, BTreeMap::new());
}

fn first_reference(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?;
    if token.chars().next()?.is_ascii_alphabetic() {
        Some(token.trim_matches(':').to_string())
    } else {
        None
    }
}

fn split_set(statement: &str) -> Option<(String, String)> {
    let (left, right) = statement.split_once('=')?;
    Some((left.trim().to_string(), right.trim().to_string()))
}

fn split_append(statement: &str) -> Option<(String, String)> {
    let (left, right) = statement.split_once("+=")?;
    Some((left.trim().to_string(), right.trim().to_string()))
}

fn looks_like_reference(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    (value.contains('.') || first == '_' || first.is_ascii_lowercase())
        && value
            .chars()
            .all(|ch| ch == '_' || ch == '.' || ch.is_ascii_alphanumeric())
}

fn state_type_values(type_name: &str) -> Vec<&str> {
    type_name
        .strip_prefix("State<")
        .and_then(|value| value.strip_suffix('>'))
        .map(|states| {
            states
                .split(',')
                .map(str::trim)
                .filter(|state| !state.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
