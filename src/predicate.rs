use std::collections::BTreeMap;

use crate::collections::collection_path_value_with;
use crate::expression::{self, DecimalNumber, MoneyAmount};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateComparison {
    pub left: String,
    pub operator: String,
    pub right: String,
}

pub fn evaluate(
    predicate: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    let predicate = strip_enclosing_parentheses(predicate.trim());
    let disjunctions = split_logical_operator(predicate, "or");
    if disjunctions.len() > 1 {
        return disjunctions
            .iter()
            .any(|part| evaluate(part, state, outputs));
    }
    let conjunctions = split_logical_operator(predicate, "and");
    if conjunctions.len() > 1 {
        return conjunctions
            .iter()
            .all(|part| evaluate(part, state, outputs));
    }
    if let Some(inner) = predicate.strip_prefix("not ")
        && !inner.trim().is_empty()
    {
        return !evaluate(inner, state, outputs);
    }
    if let Some(reference) = predicate.strip_suffix(" exists") {
        return lookup_value(reference.trim(), state, outputs).is_some();
    }

    if let Some(comparison) = simple_predicate_comparison(predicate) {
        let left = evaluate_operand(&comparison.left, state, outputs);
        let right = evaluate_operand(&comparison.right, state, outputs);
        return match comparison.operator.as_str() {
            "is" => left == right,
            "is not" => left != right,
            "contains" => expression::contains_value(&left, &right),
            "==" | "!=" | ">" | "<" | ">=" | "<=" => {
                expression::compare_values(&left, &comparison.operator, &right)
            }
            _ => false,
        };
    }
    false
}

pub fn references(predicate: &str) -> Vec<String> {
    let mut references = Vec::new();
    collect_references(predicate, &mut references);
    references
}

pub fn comparisons(predicate: &str) -> Vec<PredicateComparison> {
    let mut comparisons = Vec::new();
    collect_comparisons(predicate, &mut comparisons);
    comparisons
}

fn collect_references(predicate: &str, references: &mut Vec<String>) {
    let predicate = strip_enclosing_parentheses(predicate.trim());
    let disjunctions = split_logical_operator(predicate, "or");
    if disjunctions.len() > 1 {
        for part in disjunctions {
            collect_references(part, references);
        }
        return;
    }
    let conjunctions = split_logical_operator(predicate, "and");
    if conjunctions.len() > 1 {
        for part in conjunctions {
            collect_references(part, references);
        }
        return;
    }
    if let Some(inner) = predicate.strip_prefix("not ")
        && !inner.trim().is_empty()
    {
        collect_references(inner, references);
        return;
    }

    references.extend(simple_predicate_references(predicate));
}

fn collect_comparisons(predicate: &str, comparisons: &mut Vec<PredicateComparison>) {
    let predicate = strip_enclosing_parentheses(predicate.trim());
    let disjunctions = split_logical_operator(predicate, "or");
    if disjunctions.len() > 1 {
        for part in disjunctions {
            collect_comparisons(part, comparisons);
        }
        return;
    }
    let conjunctions = split_logical_operator(predicate, "and");
    if conjunctions.len() > 1 {
        for part in conjunctions {
            collect_comparisons(part, comparisons);
        }
        return;
    }
    if let Some(inner) = predicate.strip_prefix("not ")
        && !inner.trim().is_empty()
    {
        collect_comparisons(inner, comparisons);
        return;
    }

    if let Some(comparison) = simple_predicate_comparison(predicate) {
        comparisons.push(comparison);
    }
}

fn simple_predicate_references(predicate: &str) -> Vec<String> {
    if let Some(reference) = predicate.strip_suffix(" exists") {
        return vec![reference.trim().to_string()];
    }

    let Some(comparison) = simple_predicate_comparison(predicate) else {
        return Vec::new();
    };
    let mut references = predicate_operand_references(&comparison.left, true);
    references.extend(predicate_operand_references(&comparison.right, false));
    references
}

fn simple_predicate_comparison(predicate: &str) -> Option<PredicateComparison> {
    if predicate.strip_suffix(" exists").is_some() {
        return None;
    }

    let (operator_start, operator, operator_end) = find_comparison_operator(predicate)?;
    Some(PredicateComparison {
        left: predicate[..operator_start].trim().to_string(),
        operator: operator.to_string(),
        right: predicate[operator_end..].trim().to_string(),
    })
}

fn find_comparison_operator(predicate: &str) -> Option<(usize, &'static str, usize)> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut in_string = false;
    let mut previous_was_escape = false;

    for (index, ch) in predicate.char_indices() {
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
            _ if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                if predicate[index..].starts_with("is not")
                    && has_logical_boundary(predicate, index, index + "is not".len())
                {
                    return Some((index, "is not", index + "is not".len()));
                }
                for operator in [">=", "<=", "==", "!=", ">", "<"] {
                    if predicate[index..].starts_with(operator) {
                        return Some((index, operator, index + operator.len()));
                    }
                }
                if predicate[index..].starts_with("contains")
                    && has_logical_boundary(predicate, index, index + "contains".len())
                {
                    return Some((index, "contains", index + "contains".len()));
                }
                if predicate[index..].starts_with("is")
                    && has_logical_boundary(predicate, index, index + "is".len())
                {
                    return Some((index, "is", index + "is".len()));
                }
            }
            _ => {}
        }
    }
    None
}

fn predicate_operand_references(operand: &str, default_reference: bool) -> Vec<String> {
    if predicate_operand_looks_like_computed_expression(operand) {
        return expression::references(operand);
    }
    if default_reference || is_reference_operand(operand) {
        return vec![operand.trim().to_string()];
    }
    Vec::new()
}

fn predicate_operand_looks_like_computed_expression(operand: &str) -> bool {
    let operand = operand.trim();
    if expression::is_time_literal(operand) || expression::exact_duration_seconds(operand).is_some()
    {
        return false;
    }
    expression::arithmetic_expression(operand).is_some_and(|expression| {
        let has_named_operator = operand.contains(['+', '*', '/']) || operand.contains(" - ");
        let has_expression_term = expression
            .operands
            .iter()
            .any(|operand| operand_looks_like_expression_term(operand));
        let has_named_term = expression
            .operands
            .iter()
            .any(|operand| operand_looks_like_named_expression_term(operand));
        expression.has_operator && (has_expression_term || (has_named_operator && has_named_term))
    })
}

fn operand_looks_like_expression_term(operand: &str) -> bool {
    let operand = operand.trim();
    operand.starts_with('"')
        || operand.parse::<i64>().is_ok()
        || DecimalNumber::parse(operand).is_some()
        || MoneyAmount::parse(operand).is_some()
        || is_reference_operand(operand)
}

fn operand_looks_like_named_expression_term(operand: &str) -> bool {
    operand
        .trim()
        .chars()
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_lowercase())
}

fn is_reference_operand(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty()
        || value.parse::<f64>().is_ok()
        || value.starts_with(['"', '{', '['])
        || value.contains(':')
    {
        return false;
    }
    value.contains('.') || value.contains('[')
}

fn strip_enclosing_parentheses(mut predicate: &str) -> &str {
    loop {
        let trimmed = predicate.trim();
        if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
            return trimmed;
        }
        if !outer_parentheses_wrap(trimmed) {
            return trimmed;
        }
        predicate = &trimmed[1..trimmed.len() - 1];
    }
}

fn outer_parentheses_wrap(text: &str) -> bool {
    let mut depth = 0usize;
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
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && index != text.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn split_logical_operator<'a>(predicate: &'a str, operator: &str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut in_string = false;
    let mut previous_was_escape = false;

    for (index, ch) in predicate.char_indices() {
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
                    && predicate[index..].starts_with(operator)
                    && has_logical_boundary(predicate, index, index + operator.len())
                {
                    parts.push(predicate[start..index].trim());
                    start = index + operator.len();
                }
            }
        }
    }

    if parts.is_empty() {
        vec![predicate]
    } else {
        parts.push(predicate[start..].trim());
        parts
    }
}

fn has_logical_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(char::is_whitespace) && after.is_none_or(char::is_whitespace)
}

fn lookup_value(
    name: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    if let Some(value) = collection_path_value_with(state, name, |key| {
        outputs
            .get(key)
            .cloned()
            .or_else(|| state.get(key).cloned())
    }) {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_container_count(name, |key| lookup_value(key, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_option_value_lookup(name, |key| lookup_value(key, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_result_variant_lookup(name, |key| lookup_value(key, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_object_field_lookup(name, |key| lookup_value(key, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_map_lookup(name, |key| lookup_value(key, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_list_lookup(name, |key| lookup_value(key, state, outputs))
    {
        return Some(value);
    }
    state.get(name).or_else(|| outputs.get(name)).cloned()
}

fn resolve_operand(
    token: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> String {
    lookup_value(token, state, outputs).unwrap_or_else(|| token.to_string())
}

fn evaluate_operand(
    operand: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> String {
    let operand = operand.trim();
    if predicate_operand_looks_like_computed_expression(operand) {
        return expression::evaluate(operand, |reference| lookup_value(reference, state, outputs));
    }
    if operand.starts_with('"') && operand.ends_with('"') {
        return operand.trim_matches('"').to_string();
    }
    resolve_operand(operand, state, outputs)
}
