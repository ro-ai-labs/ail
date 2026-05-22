use crate::rif_model::RifDocument;

pub fn explain_intent(document: &RifDocument) -> String {
    let intent = &document.intent;
    let mut sentences = Vec::new();

    if intent.requires.is_empty() {
        sentences.push(format!(
            "{} starts without explicit preconditions.",
            intent.name
        ));
    } else {
        let requirements = intent
            .requires
            .iter()
            .map(|requirement| requirement.text.as_str())
            .collect::<Vec<_>>()
            .join(" and ");
        sentences.push(format!("{} first checks {}.", intent.name, requirements));
    }

    for (index, step) in intent.steps.iter().enumerate() {
        let prefix = if index == 0 { "It" } else { "Then it" };
        let mut detail = format!("{prefix} {}", third_person(&step.title));
        if let Some(call) = &step.call {
            detail.push_str(&format!(" by calling {}", call.target));
        }
        if !step.outputs.is_empty() {
            detail.push_str(&format!(
                " and produces {}",
                step.outputs.keys().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        if !step.changes.is_empty() {
            detail.push_str(&format!(" while changing {}", step.changes.join(", ")));
        }
        if !step.may_fail.is_empty() {
            detail.push_str(&format!("; it may fail with {}", step.may_fail.join(", ")));
        }
        detail.push('.');
        sentences.push(detail);
    }

    for handler in &intent.failure_handlers {
        sentences.push(format!(
            "If {}, the process will {}.",
            handler.condition,
            handler.actions.join(", then ")
        ));
    }

    for guarantee in &intent.guarantees {
        for statement in &guarantee.statements {
            sentences.push(format!("When the intent succeeds, {}.", statement));
        }
    }

    sentences.join(" ")
}

fn third_person(title: &str) -> String {
    let mut words = title.split_whitespace();
    let Some(verb) = words.next() else {
        return String::new();
    };
    let rendered = match verb.to_ascii_lowercase().as_str() {
        "verify" => "verifies".to_string(),
        "hash" => "hashes".to_string(),
        "store" => "stores".to_string(),
        "send" => "sends".to_string(),
        "reserve" => "reserves".to_string(),
        "capture" => "captures".to_string(),
        "create" => "creates".to_string(),
        "mark" => "marks".to_string(),
        other => format!("{other}s"),
    };
    let rest = words.collect::<Vec<_>>().join(" ");
    if rest.is_empty() {
        rendered
    } else {
        format!("{rendered} {rest}")
    }
}
