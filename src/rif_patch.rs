use crate::rif_model::{FailureCase, FieldDefinition, Guarantee, RifDocument};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RifPatch {
    pub target: PatchTarget,
    pub changes: Vec<PatchChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchTarget {
    Intent(String),
    Application(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchChange {
    AddFailureHandler {
        condition: String,
        actions: Vec<String>,
        stop_failure: Option<String>,
    },
    AddGuarantee {
        condition: String,
        statements: Vec<String>,
    },
    AddThingField {
        thing_name: String,
        field_name: String,
        type_name: String,
        is_secret: bool,
    },
}

pub fn parse_rif_patch(text: &str) -> Result<RifPatch, String> {
    let lines = dedent_lines(text);
    let mut target = None;
    let mut changes = Vec::new();
    let mut section: Option<String> = None;
    let mut current_change: Option<PatchChange> = None;

    for raw_line in &lines {
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with("```") {
            continue;
        }
        if stripped.starts_with("patch ") {
            continue;
        }
        if stripped == "target:" || stripped == "change:" || stripped == "add guarantee:" {
            if stripped == "add guarantee:" {
                if let Some(change) = current_change.take() {
                    changes.push(change);
                }
                section = Some("add guarantee".to_string());
            } else {
                if let Some(change) = current_change.take() {
                    changes.push(change);
                }
                section = Some(stripped.trim_end_matches(':').to_string());
            }
            continue;
        }

        match section.as_deref() {
            Some("target") => {
                if let Some(intent_name) = stripped.strip_prefix("intent ") {
                    target = Some(PatchTarget::Intent(intent_name.trim().to_string()));
                } else if let Some(app_name) = stripped.strip_prefix("app ") {
                    target = Some(PatchTarget::Application(app_name.trim().to_string()));
                }
            }
            Some("change") => {
                if let Some(step_name) = stripped
                    .strip_prefix("when step \"")
                    .and_then(|text| text.split_once("\" fails:"))
                    .map(|(title, _)| title.trim().to_string())
                {
                    if let Some(change) = current_change.take() {
                        changes.push(change);
                    }
                    current_change = Some(PatchChange::AddFailureHandler {
                        condition: format!("step \"{step_name}\" fails"),
                        actions: Vec::new(),
                        stop_failure: None,
                    });
                    continue;
                }
                if let Some(field_spec) = stripped.strip_prefix("add field ")
                    && let Some((path, type_name)) = field_spec.split_once(':')
                    && let Some((thing_name, field_name)) = path.trim().split_once('.')
                {
                    if let Some(change) = current_change.take() {
                        changes.push(change);
                    }
                    current_change = Some(PatchChange::AddThingField {
                        thing_name: thing_name.trim().to_string(),
                        field_name: field_name.trim().to_string(),
                        type_name: type_name.trim().to_string(),
                        is_secret: false,
                    });
                    continue;
                }
                if let Some(PatchChange::AddFailureHandler {
                    actions,
                    stop_failure,
                    ..
                }) = current_change.as_mut()
                {
                    actions.push(stripped.to_string());
                    if let Some(stop) = stripped.strip_prefix("stop with ") {
                        *stop_failure = Some(stop.trim().to_string());
                    }
                }
            }
            Some("add guarantee") => {
                if let Some(condition) = stripped
                    .strip_prefix("if ")
                    .and_then(|text| text.strip_suffix(':'))
                    .map(|text| text.trim().to_string())
                {
                    if let Some(change) = current_change.take() {
                        changes.push(change);
                    }
                    current_change = Some(PatchChange::AddGuarantee {
                        condition,
                        statements: Vec::new(),
                    });
                    continue;
                }
                if let Some(PatchChange::AddGuarantee { statements, .. }) = current_change.as_mut()
                {
                    statements.push(stripped.to_string());
                }
            }
            _ => {}
        }
    }

    if let Some(change) = current_change.take() {
        changes.push(change);
    }

    let target = target.ok_or_else(|| "patch must declare target".to_string())?;
    if changes.is_empty() {
        return Err("patch must declare at least one change".to_string());
    }
    Ok(RifPatch { target, changes })
}

pub fn apply_rif_patch(document: &RifDocument, patch: &RifPatch) -> Result<RifDocument, String> {
    let mut document = document.clone();
    for change in &patch.changes {
        match change {
            PatchChange::AddFailureHandler {
                condition,
                actions,
                stop_failure,
            } => {
                let PatchTarget::Intent(target_intent) = &patch.target else {
                    return Err("failure-handler patches must target an intent".to_string());
                };
                let Some(intent) = document
                    .intents
                    .iter_mut()
                    .find(|intent| intent.name == *target_intent)
                else {
                    return Err(format!("unknown intent '{target_intent}'"));
                };
                intent.failure_handlers.push(FailureCase {
                    condition: condition.clone(),
                    actions: actions.clone(),
                    stop_failure: stop_failure.clone(),
                    ignored_failures: Vec::new(),
                });
            }
            PatchChange::AddGuarantee {
                condition,
                statements,
            } => {
                let PatchTarget::Intent(target_intent) = &patch.target else {
                    return Err("guarantee patches must target an intent".to_string());
                };
                let Some(intent) = document
                    .intents
                    .iter_mut()
                    .find(|intent| intent.name == *target_intent)
                else {
                    return Err(format!("unknown intent '{target_intent}'"));
                };
                intent.guarantees.push(Guarantee {
                    conditions: vec![condition.clone()],
                    statements: statements.clone(),
                });
            }
            PatchChange::AddThingField {
                thing_name,
                field_name,
                type_name,
                is_secret,
            } => {
                let PatchTarget::Application(app_name) = &patch.target else {
                    return Err("add field patches must target an application".to_string());
                };
                if document.application.name.as_deref() != Some(app_name.as_str()) {
                    return Err(format!("unknown application target '{app_name}'"));
                }
                let Some(thing) = document.application.things.get_mut(thing_name) else {
                    return Err(format!("unknown thing '{thing_name}'"));
                };
                thing.fields.insert(
                    field_name.clone(),
                    FieldDefinition {
                        name: field_name.clone(),
                        type_name: type_name.clone(),
                        is_secret: *is_secret,
                    },
                );
            }
        }
    }
    if let PatchTarget::Intent(target_intent) = &patch.target
        && let Some(first_intent) = document.intents.first().cloned()
        && first_intent.name == *target_intent
    {
        document.intent = first_intent;
    }
    Ok(document)
}

fn dedent_lines(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let indent = lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() {
                None
            } else {
                Some(line.len() - trimmed.len())
            }
        })
        .min()
        .unwrap_or(0);
    lines
        .into_iter()
        .map(|line| line.chars().skip(indent).collect())
        .collect()
}
