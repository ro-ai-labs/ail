//! Explicit bounded binary64 expression and comparison syntax.
use crate::condition::IntComparison;
use crate::expression::{MAX_EXPRESSION_BYTES, MAX_EXPRESSION_DEPTH, MAX_EXPRESSION_NODES};
use crate::float_number;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}
impl FloatOperator {
    pub fn token(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
        }
    }
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "+" => Some(Self::Add),
            "-" => Some(Self::Subtract),
            "*" => Some(Self::Multiply),
            "/" => Some(Self::Divide),
            "%" => Some(Self::Remainder),
            _ => None,
        }
    }
    pub fn code(self) -> u8 {
        match self {
            Self::Add => 0,
            Self::Subtract => 1,
            Self::Multiply => 2,
            Self::Divide => 3,
            Self::Remainder => 4,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FloatExpr {
    Literal(u64),
    Field(String),
    Binary {
        op: FloatOperator,
        left: Box<Self>,
        right: Box<Self>,
    },
    Negate(Box<Self>),
}
fn error(message: impl std::fmt::Display) -> String {
    format!("AIL-FLOAT-001: {message}")
}
fn input_size(source: &str) -> Result<(), String> {
    if source.len() > MAX_EXPRESSION_BYTES {
        Err(error("floating expression input exceeds 65536 bytes"))
    } else {
        Ok(())
    }
}
pub(crate) fn field_key(key: &str) -> bool {
    key.contains('.')
        && key.split('.').all(|part| {
            let mut bytes = part.bytes();
            bytes
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
}
pub(crate) fn format_bits(bits: u64) -> String {
    let mut buffer = [0u8; 32];
    let size = float_number::format(bits, &mut buffer)
        .expect("binary64 format fits its documented 32-byte buffer");
    std::str::from_utf8(&buffer[..size])
        .expect("numeric format is ASCII")
        .into()
}
impl FloatExpr {
    pub fn parse(source: &str) -> Result<Self, String> {
        input_size(source)?;
        let mut parser = Parser {
            source,
            position: 0,
            nodes: 0,
        };
        let parsed = parser.expression(0, 0)?;
        parser.whitespace();
        if parser.position != source.len() {
            return Err(error("unexpected input after floating expression"));
        }
        Ok(parsed.expression)
    }
    pub fn from_postfix(source: &str) -> Result<Self, String> {
        input_size(source)?;
        let mut stack: Vec<Parsed> = Vec::new();
        for (index, token) in source.split_ascii_whitespace().enumerate() {
            if index >= MAX_EXPRESSION_NODES {
                return Err(error("floating expression has too many nodes"));
            }
            let parsed = if let Some(decimal) = token.strip_prefix("d:") {
                Parsed::leaf(Self::Literal(float_number::parse(decimal).map_err(error)?))
            } else if let Some(key) = token.strip_prefix("f:") {
                if !field_key(key) {
                    return Err(error("invalid dotted floating field key"));
                }
                Parsed::leaf(Self::Field(key.into()))
            } else if token == "neg" {
                Parsed::negate(
                    stack
                        .pop()
                        .ok_or_else(|| error("negation lacks its operand"))?,
                )?
            } else if let Some(op) = FloatOperator::from_token(token) {
                let right = stack
                    .pop()
                    .ok_or_else(|| error("operator lacks right operand"))?;
                let left = stack
                    .pop()
                    .ok_or_else(|| error("operator lacks left operand"))?;
                Parsed::binary(op, left, right)?
            } else {
                return Err(error("invalid floating postfix token"));
            };
            stack.push(parsed);
        }
        if stack.len() != 1 {
            return Err(error("floating postfix must produce exactly one value"));
        }
        Ok(stack.pop().unwrap().expression)
    }
    pub fn to_postfix(&self) -> String {
        let mut tokens = Vec::new();
        let mut pending = vec![(self, false)];
        while let Some((expression, visited)) = pending.pop() {
            match expression {
                Self::Literal(bits) => tokens.push(format!("d:{}", format_bits(*bits))),
                Self::Field(key) => tokens.push(format!("f:{key}")),
                Self::Binary { op, .. } if visited => tokens.push(op.token().into()),
                Self::Binary { left, right, .. } => {
                    pending.push((expression, true));
                    pending.push((right, false));
                    pending.push((left, false));
                }
                Self::Negate(_) if visited => tokens.push("neg".into()),
                Self::Negate(operand) => {
                    pending.push((expression, true));
                    pending.push((operand, false));
                }
            }
        }
        tokens.join(" ")
    }
    pub fn to_source(&self) -> String {
        enum Part<'a> {
            Expression(&'a FloatExpr),
            Text(&'a str),
        }
        let mut source = String::new();
        let mut pending = vec![Part::Expression(self)];
        while let Some(part) = pending.pop() {
            match part {
                Part::Text(text) => source.push_str(text),
                Part::Expression(Self::Literal(bits)) => source.push_str(&format_bits(*bits)),
                Part::Expression(Self::Field(key)) => source.push_str(key),
                Part::Expression(Self::Negate(operand)) => {
                    // A leading minus immediately before a literal is parsed as
                    // its sign. Preserve an explicit Negate node around literals.
                    if matches!(operand.as_ref(), Self::Literal(_)) {
                        pending.push(Part::Text(")"));
                        pending.push(Part::Expression(operand));
                        pending.push(Part::Text("-("));
                    } else {
                        pending.push(Part::Expression(operand));
                        pending.push(Part::Text("-"));
                    }
                }
                Part::Expression(Self::Binary { op, left, right }) => {
                    pending.extend([
                        Part::Text(")"),
                        Part::Expression(right),
                        Part::Text(" "),
                        Part::Text(op.token()),
                        Part::Text(" "),
                        Part::Expression(left),
                        Part::Text("("),
                    ]);
                }
            }
        }
        source
    }
    pub fn fields(&self) -> BTreeSet<String> {
        let mut fields = BTreeSet::new();
        let mut pending = vec![self];
        while let Some(expression) = pending.pop() {
            match expression {
                Self::Field(key) => {
                    fields.insert(key.clone());
                }
                Self::Binary { left, right, .. } => {
                    pending.push(right);
                    pending.push(left);
                }
                Self::Negate(operand) => pending.push(operand),
                Self::Literal(_) => {}
            }
        }
        fields
    }
    pub fn map_fields(&self, map: impl Fn(&str) -> String) -> Self {
        fn mapped(expression: &FloatExpr, map: &dyn Fn(&str) -> String) -> FloatExpr {
            match expression {
                FloatExpr::Field(key) => FloatExpr::Field(map(key)),
                FloatExpr::Literal(bits) => FloatExpr::Literal(*bits),
                FloatExpr::Binary { op, left, right } => FloatExpr::Binary {
                    op: *op,
                    left: Box::new(mapped(left, map)),
                    right: Box::new(mapped(right, map)),
                },
                FloatExpr::Negate(operand) => FloatExpr::Negate(Box::new(mapped(operand, map))),
            }
        }
        mapped(self, &map)
    }
    pub fn eval(&self, state: &BTreeMap<String, String>) -> Result<u64, String> {
        self.eval_borrowed(state)
    }
    pub fn eval_borrowed<V: AsRef<str>>(&self, state: &BTreeMap<String, V>) -> Result<u64, String> {
        let mut pending = vec![(self, false, 1)];
        let mut values = Vec::new();
        let mut nodes = 0;
        while let Some((expression, visited, depth)) = pending.pop() {
            if !visited {
                nodes += 1;
                if nodes > MAX_EXPRESSION_NODES || depth > MAX_EXPRESSION_DEPTH {
                    return Err("AIL-RUNTIME-FLOAT: expression exceeds node or depth limit".into());
                }
            }
            match expression {
                Self::Literal(bits) => values.push(float_number::normalize(*bits)),
                Self::Field(key) => {
                    let value = state
                        .get(key)
                        .ok_or("AIL-RUNTIME-FLOAT: missing floating field")?;
                    values.push(float_number::parse(value.as_ref()).map_err(|e| e.to_string())?);
                }
                Self::Binary { op, .. } if visited => {
                    let right = values
                        .pop()
                        .ok_or("AIL-RUNTIME-FLOAT: missing expression operand")?;
                    let left = values
                        .pop()
                        .ok_or("AIL-RUNTIME-FLOAT: missing expression operand")?;
                    values.push(
                        float_number::binary(op.code(), left, right).map_err(|e| e.to_string())?,
                    );
                }
                Self::Binary { left, right, .. } => {
                    pending.push((expression, true, depth));
                    pending.push((right, false, depth + 1));
                    pending.push((left, false, depth + 1));
                }
                Self::Negate(_) if visited => {
                    let value = values
                        .pop()
                        .ok_or("AIL-RUNTIME-FLOAT: missing negation operand")?;
                    values.push(float_number::normalize(value ^ (1 << 63)));
                }
                Self::Negate(operand) => {
                    pending.push((expression, true, depth));
                    pending.push((operand, false, depth + 1));
                }
            }
        }
        values
            .pop()
            .ok_or_else(|| "AIL-RUNTIME-FLOAT: empty expression".into())
    }
}
struct Parsed {
    expression: FloatExpr,
    depth: usize,
}
impl Parsed {
    fn leaf(expression: FloatExpr) -> Self {
        Self {
            expression,
            depth: 1,
        }
    }
    fn binary(op: FloatOperator, left: Self, right: Self) -> Result<Self, String> {
        let depth = left.depth.max(right.depth) + 1;
        if depth > MAX_EXPRESSION_DEPTH {
            return Err(error("floating expression nesting is too deep"));
        }
        Ok(Self {
            expression: FloatExpr::Binary {
                op,
                left: Box::new(left.expression),
                right: Box::new(right.expression),
            },
            depth,
        })
    }
    fn negate(operand: Self) -> Result<Self, String> {
        let depth = operand.depth + 1;
        if depth > MAX_EXPRESSION_DEPTH {
            return Err(error("floating expression nesting is too deep"));
        }
        Ok(Self {
            expression: FloatExpr::Negate(Box::new(operand.expression)),
            depth,
        })
    }
}
struct Parser<'a> {
    source: &'a str,
    position: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn expression(&mut self, minimum: u8, nesting: usize) -> Result<Parsed, String> {
        let mut left = self.atom(nesting)?;
        loop {
            self.whitespace();
            let Some(byte) = self.peek() else {
                break;
            };
            let (op, precedence) = match byte {
                b'+' => (FloatOperator::Add, 1),
                b'-' => (FloatOperator::Subtract, 1),
                b'*' => (FloatOperator::Multiply, 2),
                b'/' => (FloatOperator::Divide, 2),
                b'%' => (FloatOperator::Remainder, 2),
                _ => break,
            };
            if precedence < minimum {
                break;
            }
            self.position += 1;
            let right = self.expression(precedence + 1, nesting)?;
            self.node()?;
            left = Parsed::binary(op, left, right)?;
        }
        Ok(left)
    }
    fn atom(&mut self, nesting: usize) -> Result<Parsed, String> {
        if nesting >= MAX_EXPRESSION_DEPTH {
            return Err(error("floating source nesting is too deep"));
        }
        self.whitespace();
        match self.peek() {
            Some(b'(') => {
                self.position += 1;
                let parsed = self.expression(0, nesting + 1)?;
                self.whitespace();
                if self.peek() != Some(b')') {
                    return Err(error("expected closing parenthesis"));
                }
                self.position += 1;
                Ok(parsed)
            }
            Some(sign @ (b'+' | b'-')) => {
                self.position += 1;
                self.whitespace();
                if self.peek().is_some_and(|b| b.is_ascii_digit() || b == b'.') {
                    let start = self.position;
                    self.number_end();
                    let token = &self.source[start..self.position];
                    self.literal(&format!("{}{token}", sign as char))
                } else if self.source[self.position..].starts_with("inf")
                    || self.source[self.position..].starts_with("nan")
                {
                    let start = self.position;
                    self.identifier_end();
                    let token = &self.source[start..self.position];
                    if matches!(token, "inf" | "nan") {
                        self.literal(&format!("{}{token}", sign as char))
                    } else {
                        self.position = start;
                        let parsed = self.atom(nesting + 1)?;
                        if sign == b'-' {
                            self.node()?;
                            Parsed::negate(parsed)
                        } else {
                            Ok(parsed)
                        }
                    }
                } else {
                    let parsed = self.atom(nesting + 1)?;
                    if sign == b'-' {
                        self.node()?;
                        Parsed::negate(parsed)
                    } else {
                        Ok(parsed)
                    }
                }
            }
            Some(b) if b.is_ascii_digit() || b == b'.' => {
                let start = self.position;
                self.number_end();
                self.literal(&self.source[start..self.position])
            }
            Some(b) if b.is_ascii_alphabetic() || b == b'_' => {
                let start = self.position;
                self.identifier_end();
                let token = &self.source[start..self.position];
                if matches!(token, "inf" | "nan") {
                    self.literal(token)
                } else if field_key(token) {
                    let expression = FloatExpr::Field(token.into());
                    self.node()?;
                    Ok(Parsed::leaf(expression))
                } else {
                    Err(error("expected a floating literal or dotted field key"))
                }
            }
            _ => Err(error(
                "expected floating literal, field, unary sign or parenthesis",
            )),
        }
    }
    fn number_end(&mut self) {
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.position += 1;
        }
        if self.peek() == Some(b'.') {
            self.position += 1;
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.position += 1;
            }
        }
        if self.peek().is_some_and(|b| matches!(b, b'e' | b'E')) {
            self.position += 1;
            if self.peek().is_some_and(|b| matches!(b, b'+' | b'-')) {
                self.position += 1;
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.position += 1;
            }
        }
    }
    fn identifier_end(&mut self) {
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.'))
        {
            self.position += 1;
        }
    }
    fn literal(&mut self, token: &str) -> Result<Parsed, String> {
        let bits = float_number::parse(token).map_err(error)?;
        self.node()?;
        Ok(Parsed::leaf(FloatExpr::Literal(bits)))
    }
    fn node(&mut self) -> Result<(), String> {
        self.nodes += 1;
        if self.nodes > MAX_EXPRESSION_NODES {
            Err(error("floating expression has too many nodes"))
        } else {
            Ok(())
        }
    }
    fn peek(&self) -> Option<u8> {
        self.source.as_bytes().get(self.position).copied()
    }
    fn whitespace(&mut self) {
        while self.peek().is_some_and(|b| b.is_ascii_whitespace()) {
            self.position += 1;
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloatCondition {
    pub left: FloatExpr,
    pub comparison: IntComparison,
    pub right: FloatExpr,
}
impl FloatCondition {
    pub fn parse(source: &str) -> Result<Self, String> {
        input_size(source)?;
        let comparator = |b: u8| matches!(b, b'=' | b'!' | b'<' | b'>');
        let start = source
            .bytes()
            .position(comparator)
            .ok_or_else(|| error("expected one floating comparison"))?;
        let end = start
            + source.as_bytes()[start..]
                .iter()
                .take_while(|b| comparator(**b))
                .count();
        let comparison = IntComparison::from_token(&source[start..end]).map_err(error)?;
        if source[end..].bytes().any(comparator) {
            return Err(error("floating comparison chains are unsupported"));
        }
        let condition = Self {
            left: FloatExpr::parse(&source[..start])?,
            comparison,
            right: FloatExpr::parse(&source[end..])?,
        };
        condition.check_structure().map_err(error)?;
        Ok(condition)
    }
    pub fn from_postfix(left: &str, comparison: &str, right: &str) -> Result<Self, String> {
        if left
            .len()
            .saturating_add(right.len())
            .saturating_add(comparison.len())
            .saturating_add(2)
            > MAX_EXPRESSION_BYTES
        {
            return Err(error("floating condition input exceeds 65536 bytes"));
        }
        let condition = Self {
            left: FloatExpr::from_postfix(left)?,
            comparison: IntComparison::from_token(comparison).map_err(error)?,
            right: FloatExpr::from_postfix(right)?,
        };
        condition.check_structure().map_err(error)?;
        Ok(condition)
    }
    pub fn to_source(&self) -> String {
        format!(
            "{} {} {}",
            self.left.to_source(),
            self.comparison.token(),
            self.right.to_source()
        )
    }
    pub fn fields(&self) -> BTreeSet<String> {
        let mut fields = self.left.fields();
        fields.extend(self.right.fields());
        fields
    }
    pub fn eval(&self, state: &BTreeMap<String, String>) -> Result<bool, String> {
        self.eval_borrowed(state)
    }
    pub fn eval_borrowed<V: AsRef<str>>(
        &self,
        state: &BTreeMap<String, V>,
    ) -> Result<bool, String> {
        self.check_structure()
            .map_err(|e| format!("AIL-RUNTIME-FLOAT: {e}"))?;
        let left = self.left.eval_borrowed(state)?;
        let right = self.right.eval_borrowed(state)?;
        let op = match self.comparison {
            IntComparison::Equal => 0,
            IntComparison::NotEqual => 1,
            IntComparison::Less => 2,
            IntComparison::LessEqual => 3,
            IntComparison::Greater => 4,
            IntComparison::GreaterEqual => 5,
        };
        Ok(float_number::compare(op, left, right))
    }
    fn check_structure(&self) -> Result<(), &'static str> {
        let mut nodes = 1;
        let mut pending = vec![(&self.left, 2), (&self.right, 2)];
        while let Some((expression, depth)) = pending.pop() {
            nodes += 1;
            if nodes > MAX_EXPRESSION_NODES || depth > MAX_EXPRESSION_DEPTH {
                return Err("floating comparison exceeds combined node or depth limit");
            }
            match expression {
                FloatExpr::Binary { left, right, .. } => {
                    pending.push((left, depth + 1));
                    pending.push((right, depth + 1));
                }
                FloatExpr::Negate(operand) => pending.push((operand, depth + 1)),
                _ => {}
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn floating_precedence_unary_signs_and_postfix_preserve_bits() {
        for (source, expected) in [
            (".5 + 2 * 3", 6.5),
            ("-(data.value + 2) / +2", -3.0),
            ("-0", -0.0),
            ("5e-324", f64::from_bits(1)),
        ] {
            let expression = FloatExpr::parse(source).unwrap();
            let state = BTreeMap::from([("data.value".into(), "4".into())]);
            assert_eq!(
                expression.eval(&state).unwrap(),
                expected.to_bits(),
                "{source}"
            );
            assert_eq!(
                FloatExpr::from_postfix(&expression.to_postfix()).unwrap(),
                expression
            );
            assert_eq!(
                FloatExpr::parse(&expression.to_source()).unwrap(),
                expression
            );
        }
    }
    #[test]
    fn floating_expressions_reject_junk_and_bound_ast_structure() {
        for source in [
            "",
            "0x10",
            "1_000",
            "1e",
            "1e+",
            ". ",
            "1..2",
            "1 +",
            "data",
            "data..x",
            "infinity",
            "NaN",
            "1.0 true",
            "1\u{a0}+2",
        ] {
            assert!(FloatExpr::parse(source).is_err(), "{source:?}");
        }
        for source in [
            "d:1 d:2",
            "d:1 +",
            "d:1 neg neg +",
            "d:1x",
            "f:data",
            "i:1",
            "d:NaN",
        ] {
            assert!(FloatExpr::from_postfix(source).is_err(), "{source}");
        }
        assert!(FloatExpr::parse(&format!("{}0{}", "(".repeat(128), ")".repeat(128))).is_err());
        assert!(FloatExpr::parse(&format!("0{}", "+0".repeat(128))).is_err());
        assert!(FloatExpr::parse(&" ".repeat(65_537)).is_err());
    }
    #[test]
    fn floating_comparisons_follow_unordered_rules_and_combined_limits() {
        for (token, expected) in [
            ("==", false),
            ("!=", true),
            ("<", false),
            ("<=", false),
            (">", false),
            (">=", false),
        ] {
            assert_eq!(
                FloatCondition::parse(&format!("nan {token} 1"))
                    .unwrap()
                    .eval(&BTreeMap::new())
                    .unwrap(),
                expected
            );
        }
        assert!(
            FloatCondition::parse("-0 == 0")
                .unwrap()
                .eval(&BTreeMap::new())
                .unwrap()
        );
        assert!(FloatCondition::parse("1 < 2 < 3").is_err());
        assert!(FloatCondition::parse(&format!("0{} == 0", "+0".repeat(127))).is_err());
    }
    fn balanced(levels: usize) -> FloatExpr {
        let mut tree = FloatExpr::Literal(1.0f64.to_bits());
        for _ in 0..levels {
            tree = FloatExpr::Binary {
                op: FloatOperator::Add,
                left: Box::new(tree.clone()),
                right: Box::new(tree),
            };
        }
        tree
    }
    #[test]
    fn node_and_byte_limits_apply_to_public_trees_and_whole_conditions() {
        let tree = balanced(9);
        assert!(FloatExpr::from_postfix(&tree.to_postfix()).is_ok());
        assert!(tree.eval(&BTreeMap::new()).is_ok());
        let too_many = FloatExpr::Binary {
            op: FloatOperator::Add,
            left: Box::new(tree),
            right: Box::new(FloatExpr::Literal(0)),
        };
        assert!(FloatExpr::from_postfix(&too_many.to_postfix()).is_err());
        assert!(too_many.eval(&BTreeMap::new()).is_err());
        let exact = format!("{}1", " ".repeat(MAX_EXPRESSION_BYTES - 1));
        assert!(FloatExpr::parse(&exact).is_ok());
        assert!(FloatExpr::parse(&format!(" {exact}")).is_err());
        let operand = balanced(8);
        let condition = FloatCondition {
            left: operand.clone(),
            comparison: IntComparison::Equal,
            right: operand,
        };
        assert!(condition.eval(&BTreeMap::new()).unwrap());
        assert!(
            FloatCondition::from_postfix(
                &condition.left.to_postfix(),
                "==",
                &condition.right.to_postfix()
            )
            .is_ok()
        );
        let oversized = FloatCondition {
            left: too_many,
            comparison: IntComparison::Equal,
            right: FloatExpr::Literal(0),
        };
        assert!(oversized.eval(&BTreeMap::new()).is_err());
    }
    #[test]
    fn field_mapping_and_evaluation_preserve_order_and_runtime_error_families() {
        let expression = FloatExpr::parse("data.left + -(data.right)")
            .unwrap()
            .map_fields(|key| format!("child.{key}"));
        assert_eq!(
            expression.fields(),
            BTreeSet::from(["child.data.left".into(), "child.data.right".into()])
        );
        let state = BTreeMap::from([
            ("child.data.left".into(), "9.5".into()),
            ("child.data.right".into(), "2.5".into()),
        ]);
        assert_eq!(expression.eval(&state).unwrap(), 7.0f64.to_bits());
        let divide = FloatExpr::parse("data.left / data.right").unwrap();
        let state = BTreeMap::from([
            ("data.left".into(), "invalid operand contents".into()),
            ("data.right".into(), "0".into()),
        ]);
        let error = divide.eval(&state).unwrap_err();
        assert!(error.starts_with("AIL-RUNTIME-FLOAT:"));
        assert!(!error.contains("invalid operand contents"));
        let state = BTreeMap::from([
            ("data.left".into(), "1".into()),
            ("data.right".into(), "-0".into()),
        ]);
        assert!(
            divide
                .eval(&state)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-DIVZERO:")
        );
        assert_eq!(
            FloatExpr::parse("-7 % 3")
                .unwrap()
                .eval(&BTreeMap::new())
                .unwrap(),
            2.0f64.to_bits()
        );
    }
}
