use std::collections::BTreeMap;

use crate::core_model::{json_string, json_value};
use crate::expression;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CollectionPath<'a> {
    name: &'a str,
    selector: Option<&'a str>,
    suffix: &'a str,
}

pub fn collection_path_value(state: &BTreeMap<String, String>, path: &str) -> Option<String> {
    collection_path_value_with(state, path, |name| state.get(name).cloned())
}

pub fn collection_path_value_with<F>(
    state: &BTreeMap<String, String>,
    path: &str,
    mut resolve: F,
) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let parsed = parse_collection_path(path)?;
    if parsed.selector.is_none() && !has_collection_records(state, parsed.name) {
        return None;
    }
    let ids = matching_collection_ids(state, parsed.name, parsed.selector, &mut resolve)?;
    match parsed.suffix {
        "" => ids.first().cloned(),
        "count" => Some(ids.len().to_string()),
        "keys" => Some(ids.join(",")),
        "keys_json" => Some(json_string_array(&ids)),
        "records" => Some(collection_records_json(state, parsed.name, &ids)),
        "records_json" => Some(collection_records_json(state, parsed.name, &ids)),
        "record" => {
            let id = ids.first()?;
            Some(collection_record_json(state, parsed.name, id))
        }
        "record_json" => {
            let id = ids.first()?;
            Some(collection_record_json(state, parsed.name, id))
        }
        "first" => ids.first().cloned(),
        suffix if suffix.starts_with("first.") => {
            let field_path = &suffix["first.".len()..];
            let id = ids.first()?;
            collection_field_value(state, parsed.name, id, field_path)
        }
        suffix => {
            let id = ids.first()?;
            collection_field_value(state, parsed.name, id, suffix)
        }
    }
}

pub fn delete_collection_path(state: &mut BTreeMap<String, String>, path: &str) -> bool {
    let snapshot = state.clone();
    delete_collection_path_with(state, path, |name| snapshot.get(name).cloned())
}

pub fn delete_collection_path_with<F>(
    state: &mut BTreeMap<String, String>,
    path: &str,
    mut resolve: F,
) -> bool
where
    F: FnMut(&str) -> Option<String>,
{
    let Some(parsed) = parse_collection_path(path) else {
        delete_prefix(state, path);
        return true;
    };
    if parsed.suffix.is_empty() {
        if let Some(ids) =
            matching_collection_ids(state, parsed.name, parsed.selector, &mut resolve)
        {
            for id in ids {
                delete_prefix(state, &format!("{}.{}", parsed.name, id));
            }
            return true;
        }
        delete_prefix(state, path);
        return true;
    }
    false
}

pub fn collection_record_keys(state: &BTreeMap<String, String>, collection: &str) -> Vec<String> {
    let mut resolve = |name: &str| state.get(name).cloned();
    matching_collection_ids(state, collection, None, &mut resolve).unwrap_or_default()
}

pub fn collection_query_keys(state: &BTreeMap<String, String>, query: &str) -> Vec<String> {
    collection_query_keys_with(state, query, |name| state.get(name).cloned())
}

pub fn collection_query_keys_with<F>(
    state: &BTreeMap<String, String>,
    query: &str,
    mut resolve: F,
) -> Vec<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let Some(parsed) = parse_collection_path(query) else {
        return Vec::new();
    };
    matching_collection_ids(state, parsed.name, parsed.selector, &mut resolve).unwrap_or_default()
}

fn parse_collection_path(path: &str) -> Option<CollectionPath<'_>> {
    let (collection, suffix) = split_path_suffix(path);
    let Some(open) = collection.find('[') else {
        return Some(CollectionPath {
            name: collection,
            selector: None,
            suffix,
        });
    };
    let close = collection.rfind(']')?;
    if close <= open {
        return None;
    }
    Some(CollectionPath {
        name: &collection[..open],
        selector: Some(&collection[open + 1..close]),
        suffix,
    })
}

fn split_path_suffix(path: &str) -> (&str, &str) {
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

fn matching_collection_ids(
    state: &BTreeMap<String, String>,
    collection: &str,
    selector: Option<&str>,
    resolve: &mut impl FnMut(&str) -> Option<String>,
) -> Option<Vec<String>> {
    let prefix = format!("{collection}.");
    let mut ids = BTreeMap::new();
    for key in state.keys() {
        if let Some(rest) = key.strip_prefix(&prefix)
            && let Some((id, field_path)) = rest.split_once('.')
        {
            ids.entry(id.to_string())
                .or_insert_with(Vec::new)
                .push(field_path.to_string());
        }
    }
    if ids.is_empty() {
        return Some(Vec::new());
    }
    let mut ids: Vec<_> = ids
        .into_iter()
        .filter(|(id, _)| {
            selector.is_none_or(|selector| record_matches(state, collection, id, selector, resolve))
        })
        .map(|(id, _)| id)
        .collect();
    ids.sort();
    Some(ids)
}

fn has_collection_records(state: &BTreeMap<String, String>, collection: &str) -> bool {
    let prefix = format!("{collection}.");
    state.keys().any(|key| {
        key.strip_prefix(&prefix)
            .and_then(|rest| rest.split_once('.'))
            .is_some()
    })
}

fn record_matches(
    state: &BTreeMap<String, String>,
    collection: &str,
    id: &str,
    selector: &str,
    resolve: &mut impl FnMut(&str) -> Option<String>,
) -> bool {
    let selector = strip_enclosing_parentheses(selector.trim());
    let disjunctions = split_logical_operator(selector, "or");
    if disjunctions.len() > 1 {
        return disjunctions
            .iter()
            .any(|part| record_matches(state, collection, id, part, resolve));
    }
    let conjunctions = split_logical_operator(selector, "and");
    if conjunctions.len() > 1 {
        return conjunctions
            .iter()
            .all(|part| record_matches(state, collection, id, part, resolve));
    }
    if let Some(inner) = selector.strip_prefix("not ")
        && !inner.trim().is_empty()
    {
        return !record_matches(state, collection, id, inner, resolve);
    }

    let Some((field, operator, expected)) = split_selector_comparison(selector) else {
        return resolve(selector.trim()).is_some_and(|value| value == id);
    };
    let expected = resolve_selector_expected(expected.trim(), resolve);
    collection_field_value(state, collection, id, field.trim()).is_some_and(|value| {
        if operator == "contains" {
            expression::contains_value(&value, &expected)
        } else {
            expression::compare_values(&value, normalized_selector_operator(operator), &expected)
        }
    })
}

fn resolve_selector_expected(
    expected: &str,
    resolve: &mut impl FnMut(&str) -> Option<String>,
) -> String {
    if let Some(value) = resolve(expected) {
        return value;
    }
    if selector_expected_looks_like_expression(expected, resolve) {
        return expression::evaluate(expected, |operand| resolve(operand));
    }
    expected.to_string()
}

fn selector_expected_looks_like_expression(
    expected: &str,
    resolve: &mut impl FnMut(&str) -> Option<String>,
) -> bool {
    let Some(expression) = expression::arithmetic_expression(expected) else {
        return false;
    };
    expression.has_operator
        && (expected.contains(['+', '*', '/']) || expected.contains(" - "))
        && expression
            .operands
            .iter()
            .any(|operand| resolve(operand).is_some())
}

fn split_selector_comparison(selector: &str) -> Option<(&str, &str, &str)> {
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
                    && has_logical_boundary(selector, index, index + "contains".len())
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

fn normalized_selector_operator(operator: &str) -> &str {
    if operator == "=" { "==" } else { operator }
}

fn strip_enclosing_parentheses(mut selector: &str) -> &str {
    loop {
        let trimmed = selector.trim();
        if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
            return trimmed;
        }
        if !outer_parentheses_wrap(trimmed) {
            return trimmed;
        }
        selector = &trimmed[1..trimmed.len() - 1];
    }
}

fn outer_parentheses_wrap(text: &str) -> bool {
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

fn split_logical_operator<'a>(selector: &'a str, operator: &str) -> Vec<&'a str> {
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
                    && has_logical_boundary(selector, index, index + operator.len())
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

fn has_logical_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(char::is_whitespace) && after.is_none_or(char::is_whitespace)
}

fn collection_field_value(
    state: &BTreeMap<String, String>,
    collection: &str,
    id: &str,
    field_path: &str,
) -> Option<String> {
    let key = format!("{collection}.{id}.{field_path}");
    state.get(&key).cloned()
}

fn delete_prefix(state: &mut BTreeMap<String, String>, path: &str) {
    let prefix = format!("{path}.");
    state.retain(|key, _| key != path && !key.starts_with(&prefix));
}

fn json_string_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| json_string(value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn collection_records_json(
    state: &BTreeMap<String, String>,
    collection: &str,
    ids: &[String],
) -> String {
    format!(
        "[{}]",
        ids.iter()
            .map(|id| collection_record_json(state, collection, id))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn collection_record_json(state: &BTreeMap<String, String>, collection: &str, id: &str) -> String {
    let prefix = format!("{collection}.{id}.");
    let fields = state
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix(&prefix)
                .map(|field_path| (field_path, value))
        })
        .map(|(field_path, value)| format!("{}:{}", json_string(field_path), json_value(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{fields}}}")
}
