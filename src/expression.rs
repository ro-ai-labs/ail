#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Atom(String),
    Operator(char),
    OpenParen,
    CloseParen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DecimalNumber {
    units: i128,
    scale: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoneyAmount {
    currency: String,
    amount: DecimalNumber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArithmeticNode {
    Operand(String),
    Binary {
        operator: char,
        left: Box<ArithmeticNode>,
        right: Box<ArithmeticNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArithmeticExpression {
    pub operands: Vec<String>,
    pub has_operator: bool,
}

pub fn evaluate<F>(expression: &str, mut resolve: F) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    evaluate_with(expression, &mut resolve)
}

fn evaluate_with<F>(expression: &str, resolve: &mut F) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    if let Some(value) = evaluate_list_literal(expression, resolve) {
        return value;
    }
    if let Some(value) = evaluate_map_literal(expression, resolve) {
        return value;
    }

    let tokens = tokenize(expression);
    if tokens.is_empty() {
        return String::new();
    }
    let mut parser = EvalParser {
        tokens: &tokens,
        position: 0,
        resolve,
    };
    let Some(value) = parser.parse_sum() else {
        return expression.trim().trim_matches('"').to_string();
    };
    if parser.position == tokens.len() {
        value
    } else {
        expression.trim().trim_matches('"').to_string()
    }
}

fn evaluate_list_literal<F>(expression: &str, resolve: &mut F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let items = list_literal_items(expression)?;
    let rendered_items: Vec<String> = items
        .into_iter()
        .map(|item| evaluate_aggregate_part(item, resolve))
        .collect();
    Some(format!("[{}]", rendered_items.join(",")))
}

fn evaluate_map_literal<F>(expression: &str, resolve: &mut F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let entries = map_literal_entries(expression)?;
    let rendered_entries: Vec<String> = entries
        .into_iter()
        .map(|(key, value)| {
            format!(
                "{}:{}",
                evaluate_aggregate_part(key, resolve),
                evaluate_aggregate_part(value, resolve)
            )
        })
        .collect();
    Some(format!("{{{}}}", rendered_entries.join(",")))
}

fn evaluate_aggregate_part<F>(part: &str, resolve: &mut F) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    let part = part.trim();
    if is_quoted_literal(part) {
        part.to_string()
    } else {
        evaluate_with(part, resolve)
    }
}

pub fn references(expression: &str) -> Vec<String> {
    tokenize(expression)
        .into_iter()
        .filter_map(|token| match token {
            Token::Atom(atom) if looks_like_reference(&atom) => Some(atom),
            _ => None,
        })
        .collect()
}

pub(crate) fn split_index_lookup(expression: &str) -> Option<(&str, &str)> {
    let expression = expression.trim();
    if !expression.ends_with(']') {
        return None;
    }

    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_string = false;
    let mut previous_was_escape = false;
    let mut open_index = None;

    for (index, ch) in expression.char_indices() {
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
            '[' if brace_depth == 0 && paren_depth == 0 => {
                if bracket_depth == 0 {
                    open_index = Some(index);
                }
                bracket_depth += 1;
            }
            ']' if brace_depth == 0 && paren_depth == 0 => {
                if bracket_depth == 0 {
                    return None;
                }
                bracket_depth -= 1;
            }
            '{' if bracket_depth == 0 && paren_depth == 0 => brace_depth += 1,
            '}' if bracket_depth == 0 && paren_depth == 0 => {
                brace_depth = brace_depth.saturating_sub(1)
            }
            '(' if bracket_depth == 0 && brace_depth == 0 => paren_depth += 1,
            ')' if bracket_depth == 0 && brace_depth == 0 => {
                paren_depth = paren_depth.saturating_sub(1)
            }
            _ => {}
        }
    }

    if bracket_depth != 0 || brace_depth != 0 || paren_depth != 0 || in_string {
        return None;
    }

    let open_index = open_index?;
    let base = expression[..open_index].trim();
    let key = expression[open_index + 1..expression.len() - 1].trim();
    (!base.is_empty() && !key.is_empty()).then_some((base, key))
}

pub(crate) fn split_map_lookup(expression: &str) -> Option<(&str, &str)> {
    split_index_lookup(expression)
}

pub fn resolve_map_lookup<F>(expression: &str, mut resolve: F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let (base, key_expression) = split_index_lookup(expression)?;
    let map_value = resolve(base)?;
    let key = evaluate(key_expression, |token| resolve(token));
    map_lookup_value(&map_value, &key)
}

pub fn resolve_list_lookup<F>(expression: &str, mut resolve: F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let (base, index_expression) = split_index_lookup(expression)?;
    let list_value = resolve(base)?;
    let index = evaluate(index_expression, |token| resolve(token));
    list_lookup_value(&list_value, &index)
}

pub fn resolve_object_field_lookup<F>(expression: &str, mut resolve: F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let (base, field_name) = split_last_top_level_dot(expression)?;
    let object_value = resolve(base)?;
    map_lookup_value(&object_value, field_name)
}

pub fn resolve_option_value_lookup<F>(expression: &str, mut resolve: F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let base = expression.trim().strip_suffix(".value")?.trim();
    if base.is_empty() {
        return None;
    }
    option_inner_value(&resolve(base)?)
}

pub(crate) fn split_option_value_field_projection(expression: &str) -> Option<(&str, &str)> {
    let (base, field_path) = split_top_level_projection_suffix(expression.trim(), ".value.")?;
    (!base.is_empty() && !field_path.is_empty()).then_some((base, field_path))
}

pub(crate) fn split_result_variant_projection(expression: &str) -> Option<(&str, &str)> {
    let expression = expression.trim();
    if let Some(base) = expression.strip_suffix(".success") {
        let base = base.trim();
        return (!base.is_empty()).then_some((base, "success"));
    }
    if let Some(base) = expression.strip_suffix(".failure") {
        let base = base.trim();
        return (!base.is_empty()).then_some((base, "failure"));
    }
    None
}

pub fn resolve_result_variant_lookup<F>(expression: &str, mut resolve: F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let (base, variant) = split_result_variant_projection(expression)?;
    result_variant_inner_value(&resolve(base)?, variant)
}

pub(crate) fn split_result_variant_field_projection(
    expression: &str,
) -> Option<(&str, &str, &str)> {
    let expression = expression.trim();
    if let Some((base, field_path)) = split_top_level_projection_suffix(expression, ".success.") {
        return (!base.is_empty() && !field_path.is_empty())
            .then_some((base, "success", field_path));
    }
    if let Some((base, field_path)) = split_top_level_projection_suffix(expression, ".failure.") {
        return (!base.is_empty() && !field_path.is_empty())
            .then_some((base, "failure", field_path));
    }
    None
}

pub fn resolve_container_count<F>(expression: &str, mut resolve: F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let base = expression.trim().strip_suffix(".count")?.trim();
    if base.is_empty() {
        return None;
    }
    let value = resolve(base)?;
    container_count_value(&value)
}

pub(crate) fn map_lookup_value(map_value: &str, key: &str) -> Option<String> {
    let normalized_key = literal_value(key.trim());
    for (entry_key, entry_value) in map_literal_entries(map_value)? {
        if literal_value(entry_key.trim()) == normalized_key {
            return Some(literal_value(entry_value.trim()));
        }
    }
    None
}

pub(crate) fn list_lookup_value(list_value: &str, index: &str) -> Option<String> {
    let index = list_index(index)?;
    let item = list_literal_items(list_value)?.get(index)?.trim();
    Some(literal_value(item))
}

pub(crate) fn list_literal_values(list_value: &str) -> Option<Vec<String>> {
    Some(
        list_literal_items(list_value)?
            .into_iter()
            .map(|item| literal_value(item.trim()))
            .collect(),
    )
}

pub(crate) fn map_literal_keys(map_value: &str) -> Option<Vec<String>> {
    Some(
        map_literal_entries(map_value)?
            .into_iter()
            .map(|(key, _)| literal_value(key.trim()))
            .collect(),
    )
}

pub(crate) fn container_count_value(value: &str) -> Option<String> {
    if let Some(items) = list_literal_items(value) {
        return Some(items.len().to_string());
    }
    if let Some(entries) = map_literal_entries(value) {
        return Some(entries.len().to_string());
    }
    None
}

pub(crate) fn set_map_lookup_value(map_value: &str, key: &str, value: &str) -> Option<String> {
    let inner = map_value
        .trim()
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim();
    let normalized_key = literal_value(key.trim());
    let entries = split_top_level_commas(inner);
    let quote_inserted_key = entries
        .iter()
        .filter_map(|entry| split_top_level_once(entry.trim(), ':'))
        .any(|(entry_key, _)| is_quoted_literal(entry_key.trim()));

    let mut replaced = false;
    let mut rendered_entries = Vec::new();
    for entry in entries
        .into_iter()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let (entry_key, entry_value) = split_top_level_once(entry, ':')?;
        let entry_key = entry_key.trim();
        if literal_value(entry_key) == normalized_key {
            rendered_entries.push(format!("{entry_key}:{}", value.trim()));
            replaced = true;
        } else {
            rendered_entries.push(format!("{entry_key}:{}", entry_value.trim()));
        }
    }

    if !replaced {
        rendered_entries.push(format!(
            "{}:{}",
            format_inserted_map_key(key, quote_inserted_key),
            value.trim()
        ));
    }

    Some(format!("{{{}}}", rendered_entries.join(",")))
}

pub(crate) fn set_list_lookup_value(list_value: &str, index: &str, value: &str) -> Option<String> {
    let index = list_index(index)?;
    let mut items: Vec<String> = list_literal_items(list_value)?
        .into_iter()
        .map(|item| item.trim().to_string())
        .collect();
    if index >= items.len() {
        return None;
    }
    items[index] = value.trim().to_string();
    Some(format!("[{}]", items.join(",")))
}

pub(crate) fn set_object_field_value(
    object_value: &str,
    field_name: &str,
    value: &str,
) -> Option<String> {
    if let Some((head, tail)) = split_first_top_level_dot(field_name.trim()) {
        let nested_object = map_lookup_value(object_value, head)?;
        let updated_nested = set_object_field_value(&nested_object, tail, value)?;
        return set_map_lookup_value(object_value, head, &updated_nested);
    }
    set_map_lookup_value(object_value, field_name, value)
}

pub(crate) fn set_option_value(option_value: &str, value: &str) -> Option<String> {
    let option_value = option_value.trim();
    if option_value == "None" || option_some_value(option_value).is_some() {
        return Some(format!("Some({})", value.trim()));
    }
    None
}

pub(crate) fn set_result_variant_value(
    result_value: &str,
    variant: &str,
    value: &str,
) -> Option<String> {
    let result_value = result_value.trim();
    if result_constructor_value(result_value, "Success").is_none()
        && result_constructor_value(result_value, "Failure").is_none()
    {
        return None;
    }

    match variant {
        "success" => Some(format!("Success({})", value.trim())),
        "failure" => Some(format!("Failure({})", value.trim())),
        _ => None,
    }
}

pub(crate) fn remove_map_lookup_value(map_value: &str, key: &str) -> Option<String> {
    let inner = map_value
        .trim()
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim();
    let normalized_key = literal_value(key.trim());
    let rendered_entries: Vec<String> = split_top_level_commas(inner)
        .into_iter()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let (entry_key, entry_value) = split_top_level_once(entry, ':')?;
            (literal_value(entry_key.trim()) != normalized_key)
                .then(|| format!("{}:{}", entry_key.trim(), entry_value.trim()))
        })
        .collect();
    Some(format!("{{{}}}", rendered_entries.join(",")))
}

pub(crate) fn remove_list_lookup_value(list_value: &str, index: &str) -> Option<String> {
    let index = list_index(index)?;
    let mut items: Vec<String> = list_literal_items(list_value)?
        .into_iter()
        .map(|item| item.trim().to_string())
        .collect();
    if index < items.len() {
        items.remove(index);
    }
    Some(format!("[{}]", items.join(",")))
}

pub(crate) fn compare_values(left: &str, operator: &str, right: &str) -> bool {
    if let (Some(left), Some(right)) = (MoneyAmount::parse(left), MoneyAmount::parse(right)) {
        let Some(ordering) = left.cmp_same_currency(&right) else {
            return operator == "!=";
        };
        return compare_ordering(ordering, operator);
    }
    if let (Some(left), Some(right)) = (DecimalNumber::parse(left), DecimalNumber::parse(right)) {
        return compare_ordering(left.cmp(&right), operator);
    }
    if is_time_literal(left) && is_time_literal(right) {
        return compare_ordering(left.cmp(right), operator);
    }
    if let (Some(left), Some(right)) = (exact_duration_seconds(left), exact_duration_seconds(right))
    {
        return compare_ordering(left.cmp(&right), operator);
    }
    match operator {
        "==" => left == right,
        "!=" => left != right,
        _ => false,
    }
}

pub(crate) fn contains_value(left: &str, right: &str) -> bool {
    let needle = literal_value(right.trim());
    if let Some(items) = list_literal_values(left) {
        return items.iter().any(|item| item == &needle);
    }
    if let Some(keys) = map_literal_keys(left) {
        return keys.iter().any(|key| key == &needle);
    }
    left.contains(&needle)
}

pub fn arithmetic_expression(expression: &str) -> Option<ArithmeticExpression> {
    let tokens = tokenize(expression);
    if tokens.is_empty() {
        return None;
    }
    let mut parser = ArithmeticParser {
        tokens: &tokens,
        position: 0,
        expression: ArithmeticExpression {
            operands: Vec::new(),
            has_operator: false,
        },
    };
    parser.parse_sum()?;
    (parser.position == tokens.len()).then_some(parser.expression)
}

fn compare_ordering(ordering: std::cmp::Ordering, operator: &str) -> bool {
    match operator {
        "==" => ordering.is_eq(),
        "!=" => !ordering.is_eq(),
        ">" => ordering.is_gt(),
        "<" => ordering.is_lt(),
        ">=" => !ordering.is_lt(),
        "<=" => !ordering.is_gt(),
        _ => false,
    }
}

pub(crate) fn is_time_literal(value: &str) -> bool {
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

pub(crate) fn exact_duration_seconds(value: &str) -> Option<i128> {
    let rest = value.strip_prefix('P')?;
    if rest.is_empty() {
        return None;
    }

    let mut chars = rest.chars().peekable();
    let mut in_time = false;
    let mut seen_time_marker = false;
    let mut seen_component = false;
    let mut seconds = 0i128;
    while chars.peek().is_some() {
        if chars.peek() == Some(&'T') {
            if seen_time_marker {
                return None;
            }
            seen_time_marker = true;
            in_time = true;
            chars.next();
            chars.peek()?;
            continue;
        }

        let mut digits = String::new();
        while chars.peek().is_some_and(char::is_ascii_digit) {
            digits.push(chars.next()?);
        }
        if digits.is_empty() {
            return None;
        }
        let amount = digits.parse::<i128>().ok()?;

        let unit = chars.next()?;
        let multiplier = if in_time {
            match unit {
                'H' => 60 * 60,
                'M' => 60,
                'S' => 1,
                _ => return None,
            }
        } else {
            match unit {
                'W' => 7 * 24 * 60 * 60,
                'D' => 24 * 60 * 60,
                _ => return None,
            }
        };
        seconds = seconds.checked_add(amount.checked_mul(multiplier)?)?;
        seen_component = true;
    }
    seen_component.then_some(seconds)
}

pub fn arithmetic_tree(expression: &str) -> Option<ArithmeticNode> {
    let tokens = tokenize(expression);
    if tokens.is_empty() {
        return None;
    }
    let mut parser = TreeParser {
        tokens: &tokens,
        position: 0,
    };
    let tree = parser.parse_sum()?;
    (parser.position == tokens.len()).then_some(tree)
}

fn tokenize(expression: &str) -> Vec<Token> {
    let chars: Vec<char> = expression.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch == '"' {
            let (atom, next) = read_quoted(&chars, index);
            tokens.push(Token::Atom(atom));
            index = next;
            continue;
        }
        if is_signed_number_start(&chars, index, &tokens) {
            let (atom, next) = read_number(&chars, index);
            tokens.push(Token::Atom(atom));
            index = next;
            continue;
        }
        match ch {
            '+' | '-' | '*' | '/' => {
                tokens.push(Token::Operator(ch));
                index += 1;
            }
            '(' => {
                tokens.push(Token::OpenParen);
                index += 1;
            }
            ')' => {
                tokens.push(Token::CloseParen);
                index += 1;
            }
            _ => {
                let (atom, next) = read_atom(&chars, index);
                if atom.is_empty() {
                    index += 1;
                } else {
                    tokens.push(Token::Atom(atom));
                    index = next;
                }
            }
        }
    }
    tokens
}

fn read_quoted(chars: &[char], start: usize) -> (String, usize) {
    let mut atom = String::new();
    atom.push(chars[start]);
    let mut index = start + 1;
    while index < chars.len() {
        let ch = chars[index];
        atom.push(ch);
        index += 1;
        if ch == '"' {
            break;
        }
    }
    (atom, index)
}

fn read_number(chars: &[char], start: usize) -> (String, usize) {
    let mut atom = String::new();
    let mut index = start;
    if matches!(chars[index], '+' | '-') {
        atom.push(chars[index]);
        index += 1;
    }
    while index < chars.len() && chars[index].is_ascii_digit() {
        atom.push(chars[index]);
        index += 1;
    }
    if chars.get(index) == Some(&'.')
        && chars
            .get(index + 1)
            .is_some_and(|next| next.is_ascii_digit())
    {
        atom.push('.');
        index += 1;
        while index < chars.len() && chars[index].is_ascii_digit() {
            atom.push(chars[index]);
            index += 1;
        }
    }
    (atom, index)
}

fn read_atom(chars: &[char], start: usize) -> (String, usize) {
    let mut atom = String::new();
    let mut index = start;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if bracket_depth == 0
            && brace_depth == 0
            && paren_depth == 0
            && (ch.is_whitespace() || matches!(ch, '+' | '-' | '*' | '/' | ')'))
        {
            break;
        }
        if bracket_depth == 0 && brace_depth == 0 && paren_depth == 0 && ch == '(' {
            if atom.is_empty() {
                break;
            }
            paren_depth += 1;
            atom.push(ch);
            index += 1;
            continue;
        }
        match ch {
            '"' => {
                let (quoted, next) = read_quoted(chars, index);
                atom.push_str(&quoted);
                index = next;
                continue;
            }
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            _ => {}
        }
        atom.push(ch);
        index += 1;
    }
    (atom, index)
}

fn is_signed_number_start(chars: &[char], index: usize, tokens: &[Token]) -> bool {
    matches!(chars[index], '+' | '-')
        && chars
            .get(index + 1)
            .is_some_and(|next| next.is_ascii_digit())
        && tokens
            .last()
            .is_none_or(|token| matches!(token, Token::Operator(_) | Token::OpenParen))
}

struct EvalParser<'a, F>
where
    F: FnMut(&str) -> Option<String>,
{
    tokens: &'a [Token],
    position: usize,
    resolve: &'a mut F,
}

impl<F> EvalParser<'_, F>
where
    F: FnMut(&str) -> Option<String>,
{
    fn parse_sum(&mut self) -> Option<String> {
        let mut value = self.parse_product()?;
        while let Some(operator @ ('+' | '-')) = self.peek_operator() {
            self.position += 1;
            let right = self.parse_product()?;
            value = apply_operator(value, operator, right);
        }
        Some(value)
    }

    fn parse_product(&mut self) -> Option<String> {
        let mut value = self.parse_factor()?;
        while let Some(operator @ ('*' | '/')) = self.peek_operator() {
            self.position += 1;
            let right = self.parse_factor()?;
            value = apply_operator(value, operator, right);
        }
        Some(value)
    }

    fn parse_factor(&mut self) -> Option<String> {
        match self.tokens.get(self.position)? {
            Token::Atom(atom) => {
                self.position += 1;
                Some((self.resolve)(atom).unwrap_or_else(|| literal_value(atom)))
            }
            Token::OpenParen => {
                self.position += 1;
                let value = self.parse_sum()?;
                if matches!(self.tokens.get(self.position), Some(Token::CloseParen)) {
                    self.position += 1;
                    Some(value)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn peek_operator(&self) -> Option<char> {
        match self.tokens.get(self.position) {
            Some(Token::Operator(operator)) => Some(*operator),
            _ => None,
        }
    }
}

struct ArithmeticParser<'a> {
    tokens: &'a [Token],
    position: usize,
    expression: ArithmeticExpression,
}

struct TreeParser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl ArithmeticParser<'_> {
    fn parse_sum(&mut self) -> Option<()> {
        self.parse_product()?;
        while let Some('+' | '-') = self.peek_operator() {
            self.position += 1;
            self.expression.has_operator = true;
            self.parse_product()?;
        }
        Some(())
    }

    fn parse_product(&mut self) -> Option<()> {
        self.parse_factor()?;
        while let Some('*' | '/') = self.peek_operator() {
            self.position += 1;
            self.expression.has_operator = true;
            self.parse_factor()?;
        }
        Some(())
    }

    fn parse_factor(&mut self) -> Option<()> {
        match self.tokens.get(self.position)? {
            Token::Atom(atom) => {
                self.expression.operands.push(atom.clone());
                self.position += 1;
                Some(())
            }
            Token::OpenParen => {
                self.position += 1;
                self.parse_sum()?;
                if matches!(self.tokens.get(self.position), Some(Token::CloseParen)) {
                    self.position += 1;
                    Some(())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn peek_operator(&self) -> Option<char> {
        match self.tokens.get(self.position) {
            Some(Token::Operator(operator)) => Some(*operator),
            _ => None,
        }
    }
}

impl TreeParser<'_> {
    fn parse_sum(&mut self) -> Option<ArithmeticNode> {
        let mut node = self.parse_product()?;
        while let Some(operator @ ('+' | '-')) = self.peek_operator() {
            self.position += 1;
            let right = self.parse_product()?;
            node = ArithmeticNode::Binary {
                operator,
                left: Box::new(node),
                right: Box::new(right),
            };
        }
        Some(node)
    }

    fn parse_product(&mut self) -> Option<ArithmeticNode> {
        let mut node = self.parse_factor()?;
        while let Some(operator @ ('*' | '/')) = self.peek_operator() {
            self.position += 1;
            let right = self.parse_factor()?;
            node = ArithmeticNode::Binary {
                operator,
                left: Box::new(node),
                right: Box::new(right),
            };
        }
        Some(node)
    }

    fn parse_factor(&mut self) -> Option<ArithmeticNode> {
        match self.tokens.get(self.position)? {
            Token::Atom(atom) => {
                self.position += 1;
                Some(ArithmeticNode::Operand(atom.clone()))
            }
            Token::OpenParen => {
                self.position += 1;
                let node = self.parse_sum()?;
                if matches!(self.tokens.get(self.position), Some(Token::CloseParen)) {
                    self.position += 1;
                    Some(node)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn peek_operator(&self) -> Option<char> {
        match self.tokens.get(self.position) {
            Some(Token::Operator(operator)) => Some(*operator),
            _ => None,
        }
    }
}

fn apply_operator(left: String, operator: char, right: String) -> String {
    if let (Ok(left_number), Ok(right_number)) = (left.parse::<i64>(), right.parse::<i64>()) {
        return match operator {
            '+' => (left_number + right_number).to_string(),
            '-' => (left_number - right_number).to_string(),
            '*' => (left_number * right_number).to_string(),
            '/' if right_number != 0 => (left_number / right_number).to_string(),
            _ => left,
        };
    }
    if let (Some(left_decimal), Some(right_decimal)) =
        (DecimalNumber::parse(&left), DecimalNumber::parse(&right))
    {
        let value = match operator {
            '+' => Some(left_decimal.add(&right_decimal)),
            '-' => Some(left_decimal.sub(&right_decimal)),
            '*' => Some(left_decimal.mul(&right_decimal)),
            '/' => left_decimal.div(&right_decimal),
            _ => None,
        };
        if let Some(value) = value {
            return value.to_string();
        }
    }
    if let (Some(left_money), Some(right_money)) =
        (MoneyAmount::parse(&left), MoneyAmount::parse(&right))
    {
        let value = match operator {
            '+' => left_money.add(&right_money),
            '-' => left_money.sub(&right_money),
            _ => None,
        };
        if let Some(value) = value {
            return value.to_string();
        }
    }
    if let Some(left_money) = MoneyAmount::parse(&left)
        && let Some(right_decimal) = DecimalNumber::parse(&right)
    {
        let value = match operator {
            '*' => Some(left_money.mul(&right_decimal)),
            '/' => left_money.div(&right_decimal),
            _ => None,
        };
        if let Some(value) = value {
            return value.to_string();
        }
    }
    if let Some(left_decimal) = DecimalNumber::parse(&left)
        && let Some(right_money) = MoneyAmount::parse(&right)
        && operator == '*'
    {
        return right_money.mul(&left_decimal).to_string();
    }
    match operator {
        '+' => format!("{left}{right}"),
        _ => format!("{left} {operator} {right}"),
    }
}

impl MoneyAmount {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let (currency, amount) = text.trim().split_once(':')?;
        if currency.len() != 3 || !currency.chars().all(|ch| ch.is_ascii_uppercase()) {
            return None;
        }
        Some(Self {
            currency: currency.to_string(),
            amount: DecimalNumber::parse(amount)?,
        })
    }

    fn add(&self, other: &Self) -> Option<Self> {
        self.same_currency(other).then(|| Self {
            currency: self.currency.clone(),
            amount: self.amount.add(&other.amount),
        })
    }

    fn sub(&self, other: &Self) -> Option<Self> {
        self.same_currency(other).then(|| Self {
            currency: self.currency.clone(),
            amount: self.amount.sub(&other.amount),
        })
    }

    fn mul(&self, other: &DecimalNumber) -> Self {
        Self {
            currency: self.currency.clone(),
            amount: self.amount.mul(other),
        }
    }

    fn div(&self, other: &DecimalNumber) -> Option<Self> {
        Some(Self {
            currency: self.currency.clone(),
            amount: self.amount.div(other)?,
        })
    }

    fn same_currency(&self, other: &Self) -> bool {
        self.currency == other.currency
    }

    pub(crate) fn cmp_same_currency(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.same_currency(other)
            .then(|| self.amount.cmp(&other.amount))
    }
}

impl std::fmt::Display for MoneyAmount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}", self.currency, self.amount)
    }
}

impl DecimalNumber {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let (sign, unsigned) = if let Some(rest) = text.strip_prefix('-') {
            (-1i128, rest)
        } else if let Some(rest) = text.strip_prefix('+') {
            (1i128, rest)
        } else {
            (1i128, text)
        };
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        if whole.is_empty() && fraction.is_empty() {
            return None;
        }
        if !whole.chars().all(|ch| ch.is_ascii_digit())
            || !fraction.chars().all(|ch| ch.is_ascii_digit())
        {
            return None;
        }
        let mut digits = String::new();
        digits.push_str(if whole.is_empty() { "0" } else { whole });
        digits.push_str(fraction);
        let units = digits.parse::<i128>().ok()? * sign;
        Some(
            Self {
                units,
                scale: fraction.len() as u32,
            }
            .normalized(),
        )
    }

    fn add(&self, other: &Self) -> Self {
        let scale = self.scale.max(other.scale);
        Self {
            units: self.units * pow10(scale - self.scale)
                + other.units * pow10(scale - other.scale),
            scale,
        }
        .normalized()
    }

    fn sub(&self, other: &Self) -> Self {
        let scale = self.scale.max(other.scale);
        Self {
            units: self.units * pow10(scale - self.scale)
                - other.units * pow10(scale - other.scale),
            scale,
        }
        .normalized()
    }

    fn mul(&self, other: &Self) -> Self {
        Self {
            units: self.units * other.units,
            scale: self.scale + other.scale,
        }
        .normalized()
    }

    fn div(&self, other: &Self) -> Option<Self> {
        if other.units == 0 {
            return None;
        }
        let target_scale = 6.max(self.scale.saturating_sub(other.scale));
        let numerator = self.units * pow10(other.scale + target_scale - self.scale);
        Some(
            Self {
                units: numerator / other.units,
                scale: target_scale,
            }
            .normalized(),
        )
    }

    fn normalized(mut self) -> Self {
        while self.scale > 0 && self.units % 10 == 0 {
            self.units /= 10;
            self.scale -= 1;
        }
        self
    }

    pub(crate) fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let scale = self.scale.max(other.scale);
        let left = self.units * pow10(scale - self.scale);
        let right = other.units * pow10(scale - other.scale);
        left.cmp(&right)
    }
}

impl std::fmt::Display for DecimalNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.scale == 0 {
            return write!(formatter, "{}", self.units);
        }
        let sign = if self.units < 0 { "-" } else { "" };
        let mut digits = self.units.abs().to_string();
        let scale = self.scale as usize;
        if digits.len() <= scale {
            let padding = "0".repeat(scale + 1 - digits.len());
            digits = format!("{padding}{digits}");
        }
        let split = digits.len() - scale;
        write!(formatter, "{sign}{}.{}", &digits[..split], &digits[split..])
    }
}

fn pow10(exponent: u32) -> i128 {
    10i128.pow(exponent)
}

fn literal_value(atom: &str) -> String {
    atom.trim_matches('"').to_string()
}

fn option_inner_value(option_value: &str) -> Option<String> {
    Some(literal_value(option_some_value(option_value)?.trim()))
}

fn option_some_value(value: &str) -> Option<&str> {
    value
        .trim()
        .strip_prefix("Some(")?
        .strip_suffix(')')
        .map(str::trim)
}

fn result_variant_inner_value(result_value: &str, variant: &str) -> Option<String> {
    let constructor = match variant {
        "success" => "Success",
        "failure" => "Failure",
        _ => return None,
    };
    Some(literal_value(
        result_constructor_value(result_value, constructor)?.trim(),
    ))
}

fn result_constructor_value<'a>(value: &'a str, constructor: &str) -> Option<&'a str> {
    let prefix = format!("{constructor}(");
    value
        .trim()
        .strip_prefix(&prefix)?
        .strip_suffix(')')
        .map(str::trim)
}

fn format_inserted_map_key(key: &str, quote: bool) -> String {
    let key = literal_value(key.trim());
    if quote {
        return format!("\"{}\"", escape_string_literal(&key));
    }
    key
}

fn escape_string_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn is_quoted_literal(value: &str) -> bool {
    value.starts_with('"') && value.ends_with('"')
}

fn list_index(value: &str) -> Option<usize> {
    value.trim().parse::<usize>().ok()
}

fn list_literal_items(expression: &str) -> Option<Vec<&str>> {
    let inner = expression
        .trim()
        .strip_prefix('[')?
        .strip_suffix(']')?
        .trim();
    Some(
        split_top_level_commas(inner)
            .into_iter()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn map_literal_entries(expression: &str) -> Option<Vec<(&str, &str)>> {
    let inner = expression
        .trim()
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim();
    split_top_level_commas(inner)
        .into_iter()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| split_top_level_once(entry, ':'))
        .collect()
}

fn split_top_level_once(text: &str, delimiter: char) -> Option<(&str, &str)> {
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
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            _ if ch == delimiter && bracket_depth == 0 && brace_depth == 0 && paren_depth == 0 => {
                return Some((&text[..index], &text[index + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn split_last_top_level_dot(text: &str) -> Option<(&str, &str)> {
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_string = false;
    let mut previous_was_escape = false;
    let mut last_dot = None;

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
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '.' if bracket_depth == 0 && brace_depth == 0 && paren_depth == 0 => {
                last_dot = Some(index);
            }
            _ => {}
        }
    }

    let dot = last_dot?;
    let base = text[..dot].trim();
    let field = text[dot + 1..].trim();
    (!base.is_empty() && !field.is_empty()).then_some((base, field))
}

fn split_first_top_level_dot(text: &str) -> Option<(&str, &str)> {
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
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '.' if bracket_depth == 0
                && brace_depth == 0
                && paren_depth == 0
                && index > 0
                && index + 1 < text.len() =>
            {
                return Some((text[..index].trim(), text[index + 1..].trim()));
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_projection_suffix<'a>(
    expression: &'a str,
    marker: &str,
) -> Option<(&'a str, &'a str)> {
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_string = false;
    let mut previous_was_escape = false;

    for (index, ch) in expression.char_indices() {
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
            _ => {}
        }

        if bracket_depth == 0
            && brace_depth == 0
            && paren_depth == 0
            && expression[index..].starts_with(marker)
        {
            return Some((
                expression[..index].trim(),
                expression[index + marker.len()..].trim(),
            ));
        }
    }
    None
}

fn split_top_level_commas(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
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
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if bracket_depth == 0 && brace_depth == 0 && paren_depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

fn looks_like_reference(atom: &str) -> bool {
    let trimmed = atom.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('"')
        || matches!(trimmed, "true" | "false" | "None")
        || trimmed.parse::<i64>().is_ok()
    {
        return false;
    }
    trimmed
        .chars()
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_lowercase())
}
