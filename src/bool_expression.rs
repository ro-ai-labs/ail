//! Typed, bounded Boolean predicates shared by Core, VM and native execution.
use std::collections::BTreeMap;

use crate::condition::{IntComparison, IntCondition};
use crate::expression::{IntExpr, MAX_EXPRESSION_BYTES};
use crate::float_expression::{FloatCondition, FloatExpr};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextOperand {
    Literal(String),
    Field(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoolExpr {
    Literal(bool),
    Field(String),
    Not(Box<Self>),
    And {
        left: Box<Self>,
        right: Box<Self>,
    },
    Or {
        left: Box<Self>,
        right: Box<Self>,
    },
    CompareInt(IntCondition),
    CompareFloat(FloatCondition),
    CompareText {
        left: TextOperand,
        comparison: IntComparison,
        right: TextOperand,
    },
    CompareBool {
        left: Box<Self>,
        comparison: IntComparison,
        right: Box<Self>,
    },
}

fn error(message: impl std::fmt::Display) -> String {
    format!("AIL-BOOL-001: {message}")
}
fn input_size(source: &str) -> Result<(), String> {
    if source.len() > crate::expression::MAX_EXPRESSION_BYTES {
        Err(error("Boolean expression exceeds 65536 bytes"))
    } else {
        Ok(())
    }
}
fn field(key: &str) -> Result<String, String> {
    if crate::float_expression::field_key(key) {
        Ok(key.into())
    } else {
        Err(error("expected a dotted field key"))
    }
}

// This heterogeneous traversal counts numeric and Text nodes in the same budget
// as Boolean nodes. It is iterative even for manually constructed invalid ASTs.
#[derive(Clone, Copy)]
enum Node<'a> {
    Bool(&'a BoolExpr),
    Int(&'a IntExpr),
    Float(&'a FloatExpr),
    Text(&'a TextOperand),
}
impl<'a> Node<'a> {
    fn children(self) -> Vec<Self> {
        match self {
            Self::Bool(BoolExpr::Not(value)) => vec![Self::Bool(value)],
            Self::Bool(
                BoolExpr::And { left, right }
                | BoolExpr::Or { left, right }
                | BoolExpr::CompareBool { left, right, .. },
            ) => vec![Self::Bool(left), Self::Bool(right)],
            Self::Bool(BoolExpr::CompareInt(c)) => vec![Self::Int(&c.left), Self::Int(&c.right)],
            Self::Bool(BoolExpr::CompareFloat(c)) => {
                vec![Self::Float(&c.left), Self::Float(&c.right)]
            }
            Self::Bool(BoolExpr::CompareText { left, right, .. }) => {
                vec![Self::Text(left), Self::Text(right)]
            }
            Self::Int(IntExpr::Binary { left, right, .. }) => {
                vec![Self::Int(left), Self::Int(right)]
            }
            Self::Float(FloatExpr::Binary { left, right, .. }) => {
                vec![Self::Float(left), Self::Float(right)]
            }
            Self::Float(FloatExpr::Negate(value)) => vec![Self::Float(value)],
            _ => vec![],
        }
    }
    fn required_field(self) -> Option<(&'a str, &'static str)> {
        match self {
            Self::Bool(BoolExpr::Field(key)) => Some((key, "Bool")),
            Self::Int(IntExpr::Field(key)) => Some((key, "Int")),
            Self::Float(FloatExpr::Field(key)) => Some((key, "Float")),
            Self::Text(TextOperand::Field(key)) => Some((key, "Text")),
            _ => None,
        }
    }
}
impl BoolExpr {
    pub fn parse(source: &str) -> Result<Self, String> {
        input_size(source)?;
        let mut parser = Parser {
            source,
            tokens: source_tokens(source)?,
            position: 0,
        };
        let expression = parser.expression(0, 0)?;
        if parser.position != parser.tokens.len() {
            return Err(error("unexpected input after Boolean expression"));
        }
        expression.validate()?;
        Ok(expression)
    }
    pub fn from_postfix(source: &str) -> Result<Self, String> {
        input_size(source)?;
        let mut stack: Vec<(Operand, usize)> = Vec::new();
        for (index, token) in postfix_tokens(source)?.into_iter().enumerate() {
            if index >= crate::expression::MAX_EXPRESSION_NODES {
                return Err(error("Boolean expression has too many nodes"));
            }
            let (operand, depth) = if let Some(value) = token.strip_prefix("b:") {
                (
                    Operand::Bool(Self::Literal(match value {
                        "true" => true,
                        "false" => false,
                        _ => return Err(error("invalid Boolean literal")),
                    })),
                    1,
                )
            } else if let Some(key) = token.strip_prefix("bf:") {
                (Operand::Bool(Self::Field(field(key)?)), 1)
            } else if let Some(value) = token.strip_prefix("i:") {
                (
                    Operand::Int(IntExpr::Literal(
                        crate::expression::parse_integer(value).map_err(error)?,
                    )),
                    1,
                )
            } else if let Some(key) = token.strip_prefix("if:") {
                (Operand::Int(IntExpr::Field(field(key)?)), 1)
            } else if let Some(value) = token.strip_prefix("d:") {
                (
                    Operand::Float(FloatExpr::Literal(
                        crate::float_number::parse(value).map_err(error)?,
                    )),
                    1,
                )
            } else if let Some(key) = token.strip_prefix("ff:") {
                (Operand::Float(FloatExpr::Field(field(key)?)), 1)
            } else if let Some(value) = token.strip_prefix("s:") {
                (
                    Operand::Text(TextOperand::Literal(decode_string(value)?)),
                    1,
                )
            } else if let Some(key) = token.strip_prefix("tf:") {
                (Operand::Text(TextOperand::Field(field(key)?)), 1)
            } else if matches!(token, "not" | "fneg") {
                let (value, depth) = stack
                    .pop()
                    .ok_or_else(|| error("unary operator lacks an operand"))?;
                (
                    match (token, value) {
                        ("not", Operand::Bool(value)) => Operand::Bool(Self::Not(Box::new(value))),
                        ("fneg", Operand::Float(value)) => {
                            Operand::Float(FloatExpr::Negate(Box::new(value)))
                        }
                        _ => return Err(error("unary operator has the wrong operand type")),
                    },
                    depth + 1,
                )
            } else {
                let (right, rd) = stack
                    .pop()
                    .ok_or_else(|| error("operator lacks right operand"))?;
                let (left, ld) = stack
                    .pop()
                    .ok_or_else(|| error("operator lacks left operand"))?;
                (binary_operand(token, left, right)?, ld.max(rd) + 1)
            };
            if depth > crate::expression::MAX_EXPRESSION_DEPTH {
                return Err(error("Boolean expression nesting is too deep"));
            }
            stack.push((operand, depth));
        }
        if stack.len() != 1 {
            return Err(error("postfix must produce exactly one Boolean"));
        }
        let (Operand::Bool(expression), _) = stack.pop().unwrap() else {
            return Err(error("postfix result must be Boolean"));
        };
        expression.validate()?;
        Ok(expression)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.typed_fields()?;
        input_size(&self.to_postfix())
    }
    pub fn typed_fields(&self) -> Result<BTreeMap<String, String>, String> {
        let mut fields = BTreeMap::new();
        let mut pending = vec![(Node::Bool(self), 1)];
        let mut nodes = 0;
        while let Some((node, depth)) = pending.pop() {
            nodes += 1;
            if nodes > crate::expression::MAX_EXPRESSION_NODES
                || depth > crate::expression::MAX_EXPRESSION_DEPTH
            {
                return Err(error(
                    "Boolean expression exceeds combined node or depth limit",
                ));
            }
            if let Some((key, kind)) = node.required_field() {
                field(key)?;
                if fields
                    .insert(key.to_string(), kind.to_string())
                    .is_some_and(|old| old != kind)
                {
                    return Err(error("one field requires conflicting scalar types"));
                }
            }
            pending.extend(node.children().into_iter().map(|child| (child, depth + 1)));
        }
        Ok(fields)
    }
    pub fn to_postfix(&self) -> String {
        let mut tokens = Vec::new();
        let mut pending = vec![(Node::Bool(self), false)];
        while let Some((node, visited)) = pending.pop() {
            let children = node.children();
            if !visited && !children.is_empty() {
                pending.push((node, true));
                pending.extend(children.into_iter().rev().map(|child| (child, false)));
                continue;
            }
            tokens.push(match node {
                Node::Bool(Self::Literal(value)) => format!("b:{value}"),
                Node::Bool(Self::Field(key)) => format!("bf:{key}"),
                Node::Bool(Self::Not(_)) => "not".into(),
                Node::Bool(Self::And { .. }) => "and".into(),
                Node::Bool(Self::Or { .. }) => "or".into(),
                Node::Bool(Self::CompareInt(c)) => format!("icmp:{}", c.comparison.token()),
                Node::Bool(Self::CompareFloat(c)) => format!("fcmp:{}", c.comparison.token()),
                Node::Bool(Self::CompareText { comparison, .. }) => {
                    format!("tcmp:{}", comparison.token())
                }
                Node::Bool(Self::CompareBool { comparison, .. }) => {
                    format!("bcmp:{}", comparison.token())
                }
                Node::Int(IntExpr::Literal(value)) => format!("i:{value}"),
                Node::Int(IntExpr::Field(key)) => format!("if:{key}"),
                Node::Int(IntExpr::Binary { op, .. }) => format!("i{}", int_token(*op)),
                Node::Float(FloatExpr::Literal(value)) => {
                    format!("d:{}", crate::float_expression::format_bits(*value))
                }
                Node::Float(FloatExpr::Field(key)) => format!("ff:{key}"),
                Node::Float(FloatExpr::Negate(_)) => "fneg".into(),
                Node::Float(FloatExpr::Binary { op, .. }) => format!("f{}", op.token()),
                Node::Text(TextOperand::Literal(value)) => format!("s:{}", quote(value)),
                Node::Text(TextOperand::Field(key)) => format!("tf:{key}"),
            });
        }
        tokens.join(" ")
    }
    pub fn to_source(&self) -> String {
        enum Part<'a> {
            Expression(&'a BoolExpr),
            Text(String),
        }
        let mut source = String::new();
        let mut pending = vec![Part::Expression(self)];
        while let Some(part) = pending.pop() {
            let expression = match part {
                Part::Text(text) => {
                    source.push_str(&text);
                    continue;
                }
                Part::Expression(e) => e,
            };
            match expression {
                Self::Literal(value) => source.push_str(if *value { "true" } else { "false" }),
                Self::Field(key) => source.push_str(key),
                Self::Not(value) => {
                    source.push_str("not ");
                    pending.push(Part::Expression(value));
                }
                Self::And { left, right } | Self::Or { left, right } => {
                    let op = if matches!(expression, Self::And { .. }) {
                        " and "
                    } else {
                        " or "
                    };
                    pending.extend([
                        Part::Text(")".into()),
                        Part::Expression(right),
                        Part::Text(op.into()),
                        Part::Expression(left),
                        Part::Text("(".into()),
                    ]);
                }
                Self::CompareInt(c) => {
                    source.push_str("integer ");
                    source.push_str(&c.to_source());
                }
                Self::CompareFloat(c) => {
                    source.push_str("floating ");
                    source.push_str(&c.to_source());
                }
                Self::CompareText {
                    left,
                    comparison,
                    right,
                } => {
                    source.push_str(&format!(
                        "text {} {} {}",
                        text_source(left),
                        comparison.token(),
                        text_source(right)
                    ));
                }
                Self::CompareBool {
                    left,
                    comparison,
                    right,
                } => {
                    for (operand, prefix) in [
                        (right.as_ref(), format!(" {} ", comparison.token())),
                        (left.as_ref(), "boolean ".into()),
                    ] {
                        if matches!(
                            operand,
                            Self::Literal(_) | Self::Field(_) | Self::And { .. } | Self::Or { .. }
                        ) {
                            pending.push(Part::Expression(operand));
                        } else {
                            pending.push(Part::Text(")".into()));
                            pending.push(Part::Expression(operand));
                            pending.push(Part::Text("(".into()));
                        }
                        pending.push(Part::Text(prefix));
                    }
                }
            }
        }
        if source.len() > MAX_EXPRESSION_BYTES {
            compact_source(&source)
        } else {
            source
        }
    }
    pub fn eval(&self, state: &BTreeMap<String, String>) -> Result<bool, String> {
        self.eval_borrowed(state)
    }
    /// Borrow transport directly from function frames. Static validation visits
    /// every AST node, but runtime lookups occur only in reached subtrees.
    pub fn eval_borrowed<V: AsRef<str>>(
        &self,
        state: &BTreeMap<String, V>,
    ) -> Result<bool, String> {
        self.validate()?;
        self.eval_checked(state)
    }
    fn eval_checked<V: AsRef<str>>(&self, state: &BTreeMap<String, V>) -> Result<bool, String> {
        Ok(match self {
            Self::Literal(value) => *value,
            Self::Field(key) => match state.get(key).map(AsRef::as_ref) {
                Some("true") => true,
                Some("false") => false,
                _ => return Err("AIL-RUNTIME-BOOL: missing or malformed Boolean value".into()),
            },
            Self::Not(value) => !value.eval_checked(state)?,
            Self::And { left, right } => left.eval_checked(state)? && right.eval_checked(state)?,
            Self::Or { left, right } => left.eval_checked(state)? || right.eval_checked(state)?,
            Self::CompareInt(c) => c.eval_borrowed(state)?,
            Self::CompareFloat(c) => c.eval_borrowed(state)?,
            Self::CompareText {
                left,
                comparison,
                right,
            } => {
                let left = text_value(left, state)?;
                let right = text_value(right, state)?;
                compare(left, *comparison, right)
            }
            Self::CompareBool {
                left,
                comparison,
                right,
            } => {
                let left = left.eval_checked(state)?;
                let right = right.eval_checked(state)?;
                compare(left, *comparison, right)
            }
        })
    }
    pub fn map_fields(&self, map: impl Fn(&str) -> String) -> Self {
        fn int(e: &IntExpr, map: &dyn Fn(&str) -> String) -> IntExpr {
            match e {
                IntExpr::Literal(v) => IntExpr::Literal(*v),
                IntExpr::Field(k) => IntExpr::Field(map(k)),
                IntExpr::Binary { op, left, right } => IntExpr::Binary {
                    op: *op,
                    left: Box::new(int(left, map)),
                    right: Box::new(int(right, map)),
                },
            }
        }
        fn text(e: &TextOperand, map: &dyn Fn(&str) -> String) -> TextOperand {
            match e {
                TextOperand::Literal(v) => TextOperand::Literal(v.clone()),
                TextOperand::Field(k) => TextOperand::Field(map(k)),
            }
        }
        fn mapped(e: &BoolExpr, map: &dyn Fn(&str) -> String) -> BoolExpr {
            match e {
                BoolExpr::Literal(v) => BoolExpr::Literal(*v),
                BoolExpr::Field(k) => BoolExpr::Field(map(k)),
                BoolExpr::Not(v) => BoolExpr::Not(Box::new(mapped(v, map))),
                BoolExpr::And { left, right } => BoolExpr::And {
                    left: Box::new(mapped(left, map)),
                    right: Box::new(mapped(right, map)),
                },
                BoolExpr::Or { left, right } => BoolExpr::Or {
                    left: Box::new(mapped(left, map)),
                    right: Box::new(mapped(right, map)),
                },
                BoolExpr::CompareBool {
                    left,
                    comparison,
                    right,
                } => BoolExpr::CompareBool {
                    left: Box::new(mapped(left, map)),
                    comparison: *comparison,
                    right: Box::new(mapped(right, map)),
                },
                BoolExpr::CompareText {
                    left,
                    comparison,
                    right,
                } => BoolExpr::CompareText {
                    left: text(left, map),
                    comparison: *comparison,
                    right: text(right, map),
                },
                BoolExpr::CompareInt(c) => BoolExpr::CompareInt(IntCondition {
                    left: int(&c.left, map),
                    comparison: c.comparison,
                    right: int(&c.right, map),
                }),
                BoolExpr::CompareFloat(c) => BoolExpr::CompareFloat(FloatCondition {
                    left: c.left.map_fields(map),
                    comparison: c.comparison,
                    right: c.right.map_fields(map),
                }),
            }
        }
        mapped(self, &map)
    }
    pub fn uses_float(&self) -> bool {
        let mut pending = vec![Node::Bool(self)];
        while let Some(node) = pending.pop() {
            if matches!(node, Node::Float(_)) {
                return true;
            }
            pending.extend(node.children());
        }
        false
    }
}
// Pretty spacing can exceed the byte bound for a valid predicate with a very
// long field or literal. Remove only lexically optional whitespace; preserve
// every byte inside JSON strings and separators between identifier tokens.
fn compact_source(source: &str) -> String {
    let mut compact = String::with_capacity(source.len());
    let mut quoted = false;
    let mut escaped = false;
    let mut whitespace = false;
    let mut previous = None;
    let word = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.');
    for ch in source.chars() {
        if quoted {
            compact.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            previous = Some(ch);
            continue;
        }
        if ch.is_ascii_whitespace() {
            whitespace = true;
            continue;
        }
        if whitespace && previous.is_some_and(word) && word(ch) {
            compact.push(' ');
        }
        whitespace = false;
        compact.push(ch);
        quoted = ch == '"';
        previous = Some(ch);
    }
    compact
}

fn compare<T: Ord>(left: T, comparison: IntComparison, right: T) -> bool {
    match comparison {
        IntComparison::Equal => left == right,
        IntComparison::NotEqual => left != right,
        IntComparison::Less => left < right,
        IntComparison::LessEqual => left <= right,
        IntComparison::Greater => left > right,
        IntComparison::GreaterEqual => left >= right,
    }
}
fn text_value<'a, V: AsRef<str>>(
    operand: &'a TextOperand,
    state: &'a BTreeMap<String, V>,
) -> Result<&'a str, String> {
    let value = match operand {
        TextOperand::Literal(value) => value,
        TextOperand::Field(key) => state
            .get(key)
            .ok_or("AIL-RUNTIME-TEXT: missing text field")?
            .as_ref(),
    };
    // Rust strings are already valid UTF-8; native validates byte transports.
    if value.len() > 1_048_576 {
        return Err("AIL-RUNTIME-TEXT-LIMIT: text input exceeds 1048576 bytes".into());
    }
    Ok(value)
}
fn text_source(operand: &TextOperand) -> String {
    match operand {
        TextOperand::Literal(value) => quote(value),
        TextOperand::Field(key) => key.clone(),
    }
}
fn int_token(op: crate::expression::IntOperator) -> &'static str {
    use crate::expression::IntOperator::*;
    match op {
        Add => "+",
        Subtract => "-",
        Multiply => "*",
        Divide => "/",
        Remainder => "%",
    }
}
enum Operand {
    Bool(BoolExpr),
    Int(IntExpr),
    Float(FloatExpr),
    Text(TextOperand),
}
fn binary_operand(token: &str, left: Operand, right: Operand) -> Result<Operand, String> {
    Ok(match (left, right) {
        (Operand::Bool(left), Operand::Bool(right)) => Operand::Bool(match token {
            "and" => BoolExpr::And {
                left: Box::new(left),
                right: Box::new(right),
            },
            "or" => BoolExpr::Or {
                left: Box::new(left),
                right: Box::new(right),
            },
            _ => BoolExpr::CompareBool {
                left: Box::new(left),
                comparison: comparison_token(token, "bcmp:")?,
                right: Box::new(right),
            },
        }),
        (Operand::Int(left), Operand::Int(right)) => {
            use crate::expression::IntOperator::*;
            let op = match token {
                "i+" => Some(Add),
                "i-" => Some(Subtract),
                "i*" => Some(Multiply),
                "i/" => Some(Divide),
                "i%" => Some(Remainder),
                _ => None,
            };
            if let Some(op) = op {
                Operand::Int(IntExpr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            } else {
                Operand::Bool(BoolExpr::CompareInt(IntCondition {
                    left,
                    comparison: comparison_token(token, "icmp:")?,
                    right,
                }))
            }
        }
        (Operand::Float(left), Operand::Float(right)) => {
            if let Some(op) = token
                .strip_prefix('f')
                .and_then(crate::float_expression::FloatOperator::from_token)
            {
                Operand::Float(FloatExpr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            } else {
                Operand::Bool(BoolExpr::CompareFloat(FloatCondition {
                    left,
                    comparison: comparison_token(token, "fcmp:")?,
                    right,
                }))
            }
        }
        (Operand::Text(left), Operand::Text(right)) => Operand::Bool(BoolExpr::CompareText {
            left,
            comparison: comparison_token(token, "tcmp:")?,
            right,
        }),
        _ => return Err(error("operator has incompatible operand types")),
    })
}
fn comparison_token(token: &str, prefix: &str) -> Result<IntComparison, String> {
    IntComparison::from_token(
        token
            .strip_prefix(prefix)
            .ok_or_else(|| error("invalid typed operator"))?,
    )
    .map_err(error)
}

struct Token<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}
fn string_end(source: &str, start: usize) -> Result<usize, String> {
    if source.as_bytes().get(start) != Some(&b'"') {
        return Err(error("expected a JSON string"));
    }
    let mut position = start + 1;
    while let Some(byte) = source.as_bytes().get(position) {
        match byte {
            b'"' => return Ok(position + 1),
            b'\\' => position += 2,
            _ => position += 1,
        }
    }
    Err(error("unterminated JSON string"))
}
fn postfix_tokens(source: &str) -> Result<Vec<&str>, String> {
    let mut result = Vec::new();
    let mut p = 0;
    while p < source.len() {
        if source.as_bytes()[p].is_ascii_whitespace() {
            p += 1;
            continue;
        }
        let start = p;
        if source[p..].starts_with("s:") {
            p = string_end(source, p + 2)?;
            if source
                .as_bytes()
                .get(p)
                .is_some_and(|b| !b.is_ascii_whitespace())
            {
                return Err(error("postfix tokens require whitespace separators"));
            }
        } else {
            while source
                .as_bytes()
                .get(p)
                .is_some_and(|b| !b.is_ascii_whitespace())
            {
                p += 1;
            }
        }
        result.push(&source[start..p]);
    }
    Ok(result)
}
fn source_tokens(source: &str) -> Result<Vec<Token<'_>>, String> {
    let mut tokens = Vec::new();
    let mut p = 0;
    while p < source.len() {
        let byte = source.as_bytes()[p];
        if byte.is_ascii_whitespace() {
            p += 1;
            continue;
        }
        let start = p;
        if byte == b'"' {
            p = string_end(source, p)?;
        } else if matches!(byte, b'(' | b')' | b'+' | b'-' | b'*' | b'/' | b'%') {
            p += 1;
        } else if matches!(byte, b'=' | b'!' | b'<' | b'>') {
            p += 1;
            while source
                .as_bytes()
                .get(p)
                .is_some_and(|b| matches!(b, b'=' | b'!' | b'<' | b'>'))
            {
                p += 1;
            }
        } else {
            while source.as_bytes().get(p).is_some_and(|b| {
                !b.is_ascii_whitespace()
                    && !matches!(
                        b,
                        b'(' | b')'
                            | b'+'
                            | b'-'
                            | b'*'
                            | b'/'
                            | b'%'
                            | b'='
                            | b'!'
                            | b'<'
                            | b'>'
                            | b'"'
                    )
            }) {
                p += 1;
            }
        }
        tokens.push(Token {
            text: &source[start..p],
            start,
            end: p,
        });
    }
    Ok(tokens)
}
struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token<'a>>,
    position: usize,
}
impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.position).map(|token| token.text)
    }
    fn take(&mut self) -> Result<&'a str, String> {
        let token = self
            .peek()
            .ok_or_else(|| error("missing Boolean operand"))?;
        self.position += 1;
        Ok(token)
    }
    fn expression(&mut self, minimum: u8, nesting: usize) -> Result<BoolExpr, String> {
        let mut left = self.primary(nesting)?;
        loop {
            let precedence = match self.peek() {
                Some("or") => 1,
                Some("and") => 2,
                _ => break,
            };
            if precedence < minimum {
                break;
            }
            self.position += 1;
            let right = self.expression(precedence + 1, nesting)?;
            left = if precedence == 1 {
                BoolExpr::Or {
                    left: Box::new(left),
                    right: Box::new(right),
                }
            } else {
                BoolExpr::And {
                    left: Box::new(left),
                    right: Box::new(right),
                }
            };
            left.typed_fields()?;
        }
        Ok(left)
    }
    fn primary(&mut self, nesting: usize) -> Result<BoolExpr, String> {
        if nesting >= crate::expression::MAX_EXPRESSION_DEPTH {
            return Err(error("Boolean source nesting is too deep"));
        }
        let result = match self.take()? {
            "true" => BoolExpr::Literal(true),
            "false" => BoolExpr::Literal(false),
            "not" => BoolExpr::Not(Box::new(self.primary(nesting + 1)?)),
            "(" => {
                let e = self.expression(0, nesting + 1)?;
                if self.take()? != ")" {
                    return Err(error("expected closing Boolean parenthesis"));
                }
                e
            }
            kind @ ("integer" | "floating") => {
                let left = self.numeric(nesting)?;
                let comparison = IntComparison::from_token(self.take()?).map_err(error)?;
                let right = self.numeric(nesting)?;
                if kind == "integer" {
                    BoolExpr::CompareInt(IntCondition {
                        left: IntExpr::parse(left).map_err(error)?,
                        comparison,
                        right: IntExpr::parse(right).map_err(error)?,
                    })
                } else {
                    BoolExpr::CompareFloat(FloatCondition {
                        left: FloatExpr::parse(left).map_err(error)?,
                        comparison,
                        right: FloatExpr::parse(right).map_err(error)?,
                    })
                }
            }
            "text" => {
                let left = self.text()?;
                let comparison = IntComparison::from_token(self.take()?).map_err(error)?;
                let right = self.text()?;
                BoolExpr::CompareText {
                    left,
                    comparison,
                    right,
                }
            }
            "boolean" => {
                let left = self.bool_atom(nesting)?;
                let comparison = IntComparison::from_token(self.take()?).map_err(error)?;
                let right = self.bool_atom(nesting)?;
                BoolExpr::CompareBool {
                    left: Box::new(left),
                    comparison,
                    right: Box::new(right),
                }
            }
            token => BoolExpr::Field(field(token)?),
        };
        result.typed_fields()?;
        Ok(result)
    }
    fn bool_atom(&mut self, nesting: usize) -> Result<BoolExpr, String> {
        if self.peek().is_some_and(|token| {
            matches!(token, "true" | "false" | "(") || crate::float_expression::field_key(token)
        }) {
            self.primary(nesting)
        } else {
            Err(error(
                "Boolean comparison requires an atom or grouped predicate",
            ))
        }
    }
    fn text(&mut self) -> Result<TextOperand, String> {
        let token = self.take()?;
        if token.starts_with('"') {
            Ok(TextOperand::Literal(decode_string(token)?))
        } else {
            Ok(TextOperand::Field(field(token)?))
        }
    }
    fn numeric(&mut self, nesting: usize) -> Result<&'a str, String> {
        let start = self.position;
        let mut parentheses = 0;
        while let Some(token) = self.peek() {
            if parentheses == 0
                && (matches!(token, "and" | "or" | ")") || token.starts_with(['=', '!', '<', '>']))
            {
                break;
            }
            match token {
                "(" => {
                    parentheses += 1;
                    if nesting + parentheses >= crate::expression::MAX_EXPRESSION_DEPTH {
                        return Err(error("numeric source nesting is too deep"));
                    }
                }
                ")" => parentheses -= 1,
                _ => {}
            }
            self.position += 1;
        }
        if start == self.position || parentheses != 0 {
            return Err(error("expected a complete numeric operand"));
        }
        numeric_nesting(&self.tokens[start..self.position], nesting)?;
        Ok(&self.source[self.tokens[start].start..self.tokens[self.position - 1].end])
    }
}

// Numeric parsers bound their own recursion. Include surrounding Boolean
// groups/negations as well, including unary plus that vanishes from Float ASTs.
fn numeric_nesting(tokens: &[Token<'_>], mut nesting: usize) -> Result<(), String> {
    let mut groups = Vec::new();
    let mut unary = 0;
    let mut expects_operand = true;
    for token in tokens {
        match token.text {
            "(" => {
                groups.push(nesting);
                nesting += unary + 1;
                unary = 0;
                expects_operand = true;
            }
            ")" => {
                nesting = groups.pop().unwrap_or(nesting);
                unary = 0;
                expects_operand = false;
            }
            "+" | "-" if expects_operand => unary += 1,
            "+" | "-" | "*" | "/" | "%" => {
                unary = 0;
                expects_operand = true;
            }
            value => {
                // A sign directly attached to a numeric literal is consumed by
                // the numeric atom parser without an additional recursive call.
                let signed_literal = unary > 0
                    && (value
                        .as_bytes()
                        .first()
                        .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
                        || matches!(value, "inf" | "nan"));
                if nesting + unary - usize::from(signed_literal)
                    >= crate::expression::MAX_EXPRESSION_DEPTH
                {
                    return Err(error("numeric source nesting is too deep"));
                }
                unary = 0;
                expects_operand = false;
            }
        }
    }
    Ok(())
}

fn quote(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch < '\u{20}' || ('\u{7f}'..='\u{9f}').contains(&ch) => {
                output.push_str(&format!("\\u{:04x}", ch as u32))
            }
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}
fn decode_string(source: &str) -> Result<String, String> {
    if !source.starts_with('"') || string_end(source, 0)? != source.len() {
        return Err(error("expected one strict JSON string"));
    }
    let mut output = String::new();
    let mut p = 1;
    while p < source.len() - 1 {
        let ch = source[p..].chars().next().unwrap();
        p += ch.len_utf8();
        if ch < '\u{20}' {
            return Err(error("unescaped JSON string control character"));
        }
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        let escape = *source
            .as_bytes()
            .get(p)
            .ok_or_else(|| error("incomplete JSON escape"))?;
        p += 1;
        match escape {
            b'"' => output.push('"'),
            b'\\' => output.push('\\'),
            b'/' => output.push('/'),
            b'b' => output.push('\u{8}'),
            b'f' => output.push('\u{c}'),
            b'n' => output.push('\n'),
            b'r' => output.push('\r'),
            b't' => output.push('\t'),
            b'u' => {
                let first = hex4(source, &mut p)?;
                let scalar = if (0xd800..=0xdbff).contains(&first) {
                    if source.as_bytes().get(p..p + 2) != Some(b"\\u") {
                        return Err(error("unpaired JSON high surrogate"));
                    }
                    p += 2;
                    let second = hex4(source, &mut p)?;
                    if !(0xdc00..=0xdfff).contains(&second) {
                        return Err(error("invalid JSON surrogate pair"));
                    }
                    0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00)
                } else {
                    first
                };
                output.push(
                    char::from_u32(scalar).ok_or_else(|| error("invalid JSON Unicode scalar"))?,
                );
            }
            _ => return Err(error("invalid JSON string escape")),
        }
    }
    Ok(output)
}
fn hex4(source: &str, p: &mut usize) -> Result<u32, String> {
    let bytes = source
        .as_bytes()
        .get(*p..*p + 4)
        .ok_or_else(|| error("incomplete JSON Unicode escape"))?;
    let mut result = 0;
    for byte in bytes {
        result = result * 16
            + (*byte as char)
                .to_digit(16)
                .ok_or_else(|| error("invalid JSON Unicode escape"))?;
    }
    *p += 4;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{MAX_EXPRESSION_BYTES, MAX_EXPRESSION_DEPTH, MAX_EXPRESSION_NODES};

    fn eval(source: &str) -> bool {
        BoolExpr::parse(source)
            .unwrap()
            .eval(&BTreeMap::new())
            .unwrap()
    }

    #[test]
    fn precedence_grouping_and_short_circuit_skip_reached_errors() {
        for source in [
            "true or false and false",
            "not false and true",
            "false and missing.bool or true",
            "true or integer 1 / 0 == 0",
            "false and floating missing.float > 0 or true",
            "true or text missing.text == \"x\"",
            "boolean (true or false) > (false and true)",
        ] {
            assert!(eval(source), "{source}");
        }
        assert!(!eval("(true or false) and false"));
        assert!(!eval("not not false"));
    }

    #[test]
    fn typed_comparisons_support_all_six_operators_and_numeric_trees() {
        for (op, expected) in [
            ("==", false),
            ("!=", true),
            ("<", true),
            ("<=", true),
            (">", false),
            (">=", false),
        ] {
            for operands in [
                "integer -2 {op} 3",
                "floating -2 {op} 3",
                "text \"a\" {op} \"b\"",
                "boolean false {op} true",
            ] {
                let source = operands.replace("{op}", op);
                assert_eq!(eval(&source), expected, "{source}");
            }
        }
        assert!(eval(
            "integer (1 + 2) * 3 == 9 and floating -(1 + 2) / 2 == -1.5"
        ));
        assert!(eval("floating nan != nan and floating -0 == 0"));
        assert!(!eval("floating nan < 1"));
        assert!(eval("text \"é\" < \"🙂\" and text \"e\\u0301\" != \"é\""));
    }

    #[test]
    fn quoted_strings_are_opaque_in_source_and_typed_postfix_roundtrips() {
        let source = "text \"and or not (integer data.x == 0) \\\" \\u0000 🙂\" == \"and or not (integer data.x == 0) \\\" \\u0000 🙂\" and boolean true == true";
        let expression = BoolExpr::parse(source).unwrap();
        assert!(expression.eval(&BTreeMap::new()).unwrap());
        assert_eq!(
            BoolExpr::parse(&expression.to_source()).unwrap(),
            expression
        );
        assert_eq!(
            BoolExpr::from_postfix(&expression.to_postfix()).unwrap(),
            expression
        );
        assert_eq!(
            BoolExpr::from_postfix("if:data.x i:2 i* i:3 icmp:> ff:data.y d:0 fcmp:== or")
                .unwrap()
                .typed_fields()
                .unwrap(),
            BTreeMap::from([
                ("data.x".into(), "Int".into()),
                ("data.y".into(), "Float".into())
            ])
        );
        assert!(
            BoolExpr::from_postfix("s:\"and not ) x.y\" s:\"and not ) x.y\" tcmp:==")
                .unwrap()
                .eval(&BTreeMap::new())
                .unwrap()
        );
    }

    #[test]
    fn malformed_source_and_postfix_are_rejected_without_coercion() {
        for source in [
            "",
            "true false",
            "true && false",
            "true or",
            "not",
            "1",
            "bare",
            "integer 1 < 2 < 3",
            "text data.x == 1",
            "boolean true == 1",
            "integer data.x == 1 and floating data.x == 1",
            "text \"\\ud800\" == \"x\"",
            "text \"a\n\" == \"a\"",
            "integer (1 < 2)",
            "true)",
            "(true",
            "true.and",
        ] {
            if source == "true.and" {
                assert!(BoolExpr::parse(source).is_ok());
            } else {
                assert!(BoolExpr::parse(source).is_err(), "{source:?}");
            }
        }
        for source in [
            "",
            "b:TRUE",
            "b:true b:false",
            "i:1",
            "bf:bad",
            "s:\"x\"b:true",
            "i:1 d:1 icmp:==",
            "b:true i:1 and",
            "b:true not not nope",
            "d:1 d:2 i+",
            "s:\"\\ud800\" s:\"x\" tcmp:==",
            "f:data.x",
            "b:true b:false bcmp:=",
            "bf:data.x if:data.x i:0 icmp:== and",
        ] {
            assert!(BoolExpr::from_postfix(source).is_err(), "{source:?}");
        }
    }

    #[test]
    fn reached_fields_validate_in_order_and_keep_error_families() {
        for (source, family) in [
            ("missing.bool", "BOOL"),
            ("text missing.text == \"a\"", "TEXT"),
            ("integer missing.int == 0", "INT"),
            ("floating missing.float == 0", "FLOAT"),
            ("true and integer 1 / 0 == 0", "DIVZERO"),
            ("boolean missing.bool == (integer 1 / 0 == 0)", "BOOL"),
        ] {
            let error = BoolExpr::parse(source)
                .unwrap()
                .eval(&BTreeMap::new())
                .unwrap_err();
            assert!(error.contains(&format!("AIL-RUNTIME-{family}:")), "{error}");
        }
        let e = BoolExpr::parse("data.flag").unwrap();
        for value in ["1", "True", " true", "false ", ""] {
            assert!(
                e.eval(&BTreeMap::from([("data.flag".into(), value.into())]))
                    .unwrap_err()
                    .contains("AIL-RUNTIME-BOOL:")
            );
        }
        let state = BTreeMap::from([("data.text".into(), "x".repeat(1_048_577))]);
        assert!(
            BoolExpr::parse("text data.text == missing.text")
                .unwrap()
                .eval(&state)
                .unwrap_err()
                .contains("AIL-RUNTIME-TEXT-LIMIT:")
        );
        assert!(
            BoolExpr::parse("true or text data.text == missing.text")
                .unwrap()
                .eval(&state)
                .unwrap()
        );
    }

    #[test]
    fn mapping_and_types_traverse_skipped_subtrees_and_preserve_text() {
        let e = BoolExpr::parse("data.flag or integer data.n > 0 and floating data.x == 0 and text data.s == \"data.n\"").unwrap();
        assert!(e.uses_float());
        assert_eq!(
            e.typed_fields().unwrap(),
            BTreeMap::from([
                ("data.flag".into(), "Bool".into()),
                ("data.n".into(), "Int".into()),
                ("data.s".into(), "Text".into()),
                ("data.x".into(), "Float".into())
            ])
        );
        let mapped = e.map_fields(|key| format!("pkg.{key}"));
        assert!(mapped.to_source().contains("\"data.n\""));
        assert!(
            mapped
                .typed_fields()
                .unwrap()
                .keys()
                .all(|key| key.starts_with("pkg."))
        );
        assert!(
            !BoolExpr::parse("text data.s == \"floating data.x\"")
                .unwrap()
                .uses_float()
        );
    }

    #[test]
    fn byte_node_depth_and_source_nesting_budgets_are_shared() {
        let exact = format!("{}true", " ".repeat(MAX_EXPRESSION_BYTES - 4));
        assert!(BoolExpr::parse(&exact).is_ok());
        assert!(BoolExpr::parse(&(exact + " ")).is_err());
        let nested = format!(
            "{}true{}",
            "(".repeat(MAX_EXPRESSION_DEPTH),
            ")".repeat(MAX_EXPRESSION_DEPTH)
        );
        assert!(BoolExpr::parse(&nested).is_err());
        let exact = format!("{}true", "not ".repeat(MAX_EXPRESSION_DEPTH - 1));
        let e = BoolExpr::parse(&exact).unwrap();
        assert_eq!(BoolExpr::parse(&e.to_source()).unwrap(), e);
        assert!(BoolExpr::parse(&format!("not {exact}")).is_err());
        let mut numeric = IntExpr::Literal(0);
        for _ in 0..MAX_EXPRESSION_DEPTH - 1 {
            numeric = IntExpr::Binary {
                op: crate::expression::IntOperator::Add,
                left: Box::new(numeric),
                right: Box::new(IntExpr::Literal(0)),
            };
        }
        assert!(
            BoolExpr::CompareInt(IntCondition {
                left: numeric,
                comparison: IntComparison::Equal,
                right: IntExpr::Literal(0)
            })
            .validate()
            .is_err()
        );
        let mut e = BoolExpr::Literal(true);
        for _ in 0..9 {
            e = BoolExpr::And {
                left: Box::new(e.clone()),
                right: Box::new(e),
            };
        }
        assert_eq!(MAX_EXPRESSION_NODES, 1024);
        assert!(e.validate().is_ok());
        assert!(BoolExpr::Not(Box::new(e.clone())).validate().is_ok());
        assert!(
            BoolExpr::Not(Box::new(BoolExpr::Not(Box::new(e))))
                .validate()
                .is_err()
        );
    }

    #[test]
    fn rendering_preserves_deep_mixed_boolean_comparison_trees() {
        let mut expression = BoolExpr::Literal(true);
        for _ in 0..60 {
            expression = BoolExpr::CompareBool {
                left: Box::new(BoolExpr::And {
                    left: Box::new(expression),
                    right: Box::new(BoolExpr::Literal(true)),
                }),
                comparison: IntComparison::Equal,
                right: Box::new(BoolExpr::Literal(true)),
            };
        }
        expression.validate().unwrap();
        assert_eq!(
            BoolExpr::parse(&expression.to_source()).unwrap(),
            expression
        );
        assert_eq!(
            BoolExpr::from_postfix(&expression.to_postfix()).unwrap(),
            expression
        );
    }

    #[test]
    fn normalized_numeric_unary_nesting_shares_source_budget_with_boolean_groups() {
        let source = format!(
            "{}floating {}data.x == 0{}",
            "(".repeat(64),
            "+".repeat(64),
            ")".repeat(64)
        );
        assert!(BoolExpr::parse(&source).is_err());
        let source = format!(
            "{}floating {}data.x == 0{}",
            "(".repeat(63),
            "+".repeat(64),
            ")".repeat(63)
        );
        assert!(BoolExpr::parse(&source).is_ok());
    }

    #[test]
    fn string_surrogates_controls_and_every_typed_operator_round_trip() {
        for source in [
            "s:\"\\ud83d\\ude42\\b\\f\\n\\r\\t\\/\\\\\\\"\" s:\"🙂\\u0008\\u000c\\n\\r\\t/\\\\\\\"\" tcmp:==",
            "i:8 i:2 i/ i:3 i% i:1 i+ i:2 i* i:1 i- i:3 icmp:==",
            "d:8 d:2 f/ d:3 f% d:1 f+ d:2 f* d:1 f- fneg d:-3 fcmp:==",
        ] {
            let expression = BoolExpr::from_postfix(source).unwrap();
            assert!(expression.eval(&BTreeMap::new()).unwrap(), "{source}");
            assert_eq!(
                BoolExpr::parse(&expression.to_source()).unwrap(),
                expression
            );
            assert_eq!(
                BoolExpr::from_postfix(&expression.to_postfix()).unwrap(),
                expression
            );
        }
        for value in [
            r#""\udc00""#,
            r#""\ud800\u0000""#,
            r#""\ud800x""#,
            r#""\u00xz""#,
            r#""\q""#,
        ] {
            assert!(BoolExpr::parse(&format!("text {value} == \"\"")).is_err());
        }
    }

    #[test]
    fn borrowed_frames_only_read_reached_operands_in_evaluation_order() {
        use std::cell::RefCell;
        struct Observed<'a> {
            key: &'static str,
            value: &'static str,
            reads: &'a RefCell<Vec<&'static str>>,
        }
        impl AsRef<str> for Observed<'_> {
            fn as_ref(&self) -> &str {
                self.reads.borrow_mut().push(self.key);
                self.value
            }
        }
        let reads = RefCell::new(Vec::new());
        let state = [
            ("data.flag", "true"),
            ("data.int", "7"),
            ("data.float", "0.5"),
            ("data.text", "object"),
        ]
        .into_iter()
        .map(|(key, value)| {
            (
                key.into(),
                Observed {
                    key,
                    value,
                    reads: &reads,
                },
            )
        })
        .collect();
        assert!(BoolExpr::parse("data.flag or integer data.int > 0 and floating data.float > 0 and text data.text == \"object\"").unwrap().eval_borrowed(&state).unwrap());
        assert_eq!(*reads.borrow(), ["data.flag"]);
        reads.borrow_mut().clear();
        assert!(
            BoolExpr::parse(
                "integer data.int > 0 and floating data.float > 0 and text data.text == \"object\""
            )
            .unwrap()
            .eval_borrowed(&state)
            .unwrap()
        );
        assert_eq!(*reads.borrow(), ["data.int", "data.float", "data.text"]);
        reads.borrow_mut().clear();
        let error = BoolExpr::parse("integer 1 / 0 == data.int")
            .unwrap()
            .eval_borrowed(&state)
            .unwrap_err();
        assert!(error.contains("AIL-RUNTIME-DIVZERO:"));
        assert!(reads.borrow().is_empty());
    }
}
