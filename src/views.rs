use crate::graph_builder::infer_permissions;
use crate::rif_model::RifDocument;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowItem {
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureItem {
    pub condition: String,
    pub actions: Vec<String>,
    pub stop: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionViewModel {
    pub reads: Vec<String>,
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewModel {
    pub intent: String,
    pub flow: Vec<FlowItem>,
    pub failures: Vec<FailureItem>,
    pub permissions: PermissionViewModel,
}

pub fn flow_view(document: &RifDocument) -> String {
    let items = flow_items(document);
    let mut lines = Vec::new();
    for (index, item) in items.iter().enumerate() {
        lines.push(format!("[{}]", item.label));
        if index + 1 != items.len() {
            lines.push("    ↓".to_string());
        }
    }
    lines.join("\n")
}

pub fn failure_view(document: &RifDocument) -> String {
    document
        .intent
        .failure_handlers
        .iter()
        .map(|handler| {
            let mut lines = vec![handler.condition.clone()];
            for action in &handler.actions {
                lines.push("    ↓".to_string());
                if let Some(stop) = action.strip_prefix("stop with ") {
                    lines.push(format!("Stop with {}", stop.trim()));
                } else {
                    lines.push(action.clone());
                }
            }
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn permission_view(document: &RifDocument) -> String {
    let permissions = infer_permissions(document);
    let mut reads: Vec<_> = permissions
        .iter()
        .filter(|permission| permission.kind == "Read")
        .map(|permission| permission.target.clone())
        .collect();
    let mut changes: Vec<_> = permissions
        .iter()
        .filter(|permission| permission.kind == "Change")
        .map(|permission| permission.target.clone())
        .collect();
    reads.sort();
    changes.sort();

    let mut lines = vec![document.intent.name.clone(), "  reads:".to_string()];
    lines.extend(reads.into_iter().map(|target| format!("    {target}")));
    lines.push(String::new());
    lines.push("  changes:".to_string());
    lines.extend(changes.into_iter().map(|target| format!("    {target}")));
    lines.join("\n")
}

pub fn effect_view(document: &RifDocument) -> String {
    let mut lines = vec![document.intent.name.clone(), "  effects:".to_string()];
    let mut seen = std::collections::BTreeSet::new();
    for step in &document.intent.steps {
        if let Some(call) = &step.call {
            push_effect(&mut lines, &mut seen, format!("call {}", call.target));
        }
        for target in &step.changes {
            push_effect(&mut lines, &mut seen, format!("write {target}"));
        }
        for statement in &step.set_statements {
            if let Some((left, _)) = statement.split_once('=') {
                push_effect(&mut lines, &mut seen, format!("write {}", left.trim()));
            }
        }
        for statement in &step.append_statements {
            if let Some((left, _)) = statement.split_once("+=") {
                push_effect(&mut lines, &mut seen, format!("append {}", left.trim()));
            }
        }
        for statement in &step.delete_statements {
            push_effect(
                &mut lines,
                &mut seen,
                format!("delete {}", statement.trim()),
            );
        }
    }
    lines.join("\n")
}

fn push_effect(
    lines: &mut Vec<String>,
    seen: &mut std::collections::BTreeSet<String>,
    effect: String,
) {
    if seen.insert(effect.clone()) {
        lines.push(format!("    {effect}"));
    }
}

pub fn security_view(document: &RifDocument) -> String {
    let mut secrets: Vec<_> = document
        .intent
        .subjects
        .iter()
        .chain(document.intent.inputs.iter())
        .filter(|(_, thing)| thing.is_secret)
        .map(|(name, _)| name.clone())
        .collect();
    secrets.sort();

    let mut lines = vec![document.intent.name.clone(), "  secrets:".to_string()];
    if secrets.is_empty() {
        lines.push("    none".to_string());
    } else {
        lines.extend(secrets.into_iter().map(|secret| format!("    {secret}")));
    }
    lines.join("\n")
}

pub fn view_model(document: &RifDocument) -> ViewModel {
    let permissions = infer_permissions(document);
    let mut reads: Vec<_> = permissions
        .iter()
        .filter(|permission| permission.kind == "Read")
        .map(|permission| permission.target.clone())
        .collect();
    let mut changes: Vec<_> = permissions
        .iter()
        .filter(|permission| permission.kind == "Change")
        .map(|permission| permission.target.clone())
        .collect();
    reads.sort();
    changes.sort();

    ViewModel {
        intent: document.intent.name.clone(),
        flow: flow_items(document),
        failures: document
            .intent
            .failure_handlers
            .iter()
            .map(|handler| FailureItem {
                condition: handler.condition.clone(),
                actions: handler.actions.clone(),
                stop: handler.stop_failure.clone(),
            })
            .collect(),
        permissions: PermissionViewModel { reads, changes },
    }
}

pub fn view_model_json(document: &RifDocument) -> String {
    let model = view_model(document);
    format!(
        "{{\"intent\":{},\"flow\":[{}],\"failures\":[{}],\"permissions\":{{\"reads\":[{}],\"changes\":[{}]}}}}",
        crate::core_model::json_string(&model.intent),
        model
            .flow
            .iter()
            .map(|item| format!(
                "{{\"kind\":{},\"label\":{}}}",
                crate::core_model::json_string(&item.kind),
                crate::core_model::json_string(&item.label)
            ))
            .collect::<Vec<_>>()
            .join(","),
        model
            .failures
            .iter()
            .map(|failure| format!(
                "{{\"condition\":{},\"actions\":[{}],\"stop\":{}}}",
                crate::core_model::json_string(&failure.condition),
                failure
                    .actions
                    .iter()
                    .map(|action| crate::core_model::json_string(action))
                    .collect::<Vec<_>>()
                    .join(","),
                failure
                    .stop
                    .as_ref()
                    .map(|stop| crate::core_model::json_string(stop))
                    .unwrap_or_else(|| "null".to_string())
            ))
            .collect::<Vec<_>>()
            .join(","),
        model
            .permissions
            .reads
            .iter()
            .map(|target| crate::core_model::json_string(target))
            .collect::<Vec<_>>()
            .join(","),
        model
            .permissions
            .changes
            .iter()
            .map(|target| crate::core_model::json_string(target))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn flow_items(document: &RifDocument) -> Vec<FlowItem> {
    let mut items = Vec::new();
    if let Some(start) = start_state_label(document) {
        items.push(FlowItem {
            kind: "state".to_string(),
            label: start,
        });
    }
    items.extend(document.intent.steps.iter().map(|step| FlowItem {
        kind: "step".to_string(),
        label: step.title.clone(),
    }));
    if let Some(end) = end_state_label(document) {
        items.push(FlowItem {
            kind: "state".to_string(),
            label: end,
        });
    }
    items
}

fn start_state_label(document: &RifDocument) -> Option<String> {
    for requirement in &document.intent.requires {
        let parts: Vec<_> = requirement.text.split_whitespace().collect();
        if parts.len() == 3 && parts[1] == "is" && parts[0].ends_with(".status") {
            let subject = parts[0].trim_end_matches(".status");
            let noun = document
                .intent
                .subjects
                .get(subject)
                .map(|thing| thing.type_name.as_str())
                .unwrap_or(subject);
            return Some(format!("{} {}", parts[2], noun));
        }
    }
    None
}

fn end_state_label(document: &RifDocument) -> Option<String> {
    for guarantee in &document.intent.guarantees {
        for statement in &guarantee.statements {
            let parts: Vec<_> = statement.split_whitespace().collect();
            if parts.len() == 3 && parts[1] == "is" && parts[0].ends_with(".status") {
                let subject = parts[0].trim_end_matches(".status");
                let noun = document
                    .intent
                    .subjects
                    .get(subject)
                    .map(|thing| thing.type_name.as_str())
                    .unwrap_or(subject);
                return Some(format!("{} {}", parts[2], noun));
            }
        }
    }
    None
}
