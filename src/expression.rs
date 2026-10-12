use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EXPRESSION_BYTES: usize = 65_536;
pub const MAX_EXPRESSION_NODES: usize = 1_024;
pub const MAX_EXPRESSION_DEPTH: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

impl IntOperator {
    fn token(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
        }
    }

    fn from_token(token: &str) -> Option<Self> {
        match token {
            "+" => Some(Self::Add),
            "-" => Some(Self::Subtract),
            "*" => Some(Self::Multiply),
            "/" => Some(Self::Divide),
            "%" => Some(Self::Remainder),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntExpr {
    Literal(i64),
    Field(String),
    Binary {
        op: IntOperator,
        left: Box<IntExpr>,
        right: Box<IntExpr>,
    },
}

#[derive(Debug, Eq, PartialEq)]
enum Value {
    Int(i64),
    Text(String),
}

impl Value {
    fn require_int(self) -> Result<i64, String> {
        match self {
            Self::Int(value) => Ok(value),
            Self::Text(_) => Err("AIL-RUNTIME-INT: arithmetic requires an Int operand".into()),
        }
    }

    /// Explicit conversion at the declared-Int field transport boundary only.
    fn decode_declared_int(self) -> Result<Self, String> {
        match self {
            Self::Text(text) => parse_integer(&text).map(Self::Int),
            integer @ Self::Int(_) => Ok(integer),
        }
    }
}

/// Decode the string transport representation of an explicitly declared Int.
/// Whitespace, separators, non-ASCII digits, and values outside i64 are errors.
pub fn parse_integer(text: &str) -> Result<i64, String> {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("AIL-RUNTIME-INT: expected a signed decimal integer".into());
    }
    text.parse::<i64>()
        .map_err(|_| "AIL-RUNTIME-INT: integer is outside the signed 64-bit range".into())
}

impl IntExpr {
    pub fn parse(source: &str) -> Result<Self, String> {
        check_source_size(source)?;
        let mut parser = InfixParser {
            source,
            position: 0,
            nodes: 0,
        };
        let parsed = parser.expression(0, 0)?;
        parser.skip_whitespace();
        if parser.position != source.len() {
            return Err("AIL-EXPR: unexpected input after expression".into());
        }
        Ok(parsed.expression)
    }

    pub fn from_postfix(source: &str) -> Result<Self, String> {
        check_source_size(source)?;
        let mut stack: Vec<ParsedExpr> = Vec::new();
        for (index, token) in source.split_ascii_whitespace().enumerate() {
            if index >= MAX_EXPRESSION_NODES {
                return Err("AIL-EXPR: expression has too many nodes".into());
            }
            let parsed = if let Some(integer) = token.strip_prefix("i:") {
                ParsedExpr::leaf(Self::Literal(parse_integer(integer)?))
            } else if let Some(field) = token.strip_prefix("f:") {
                if !is_field_key(field) {
                    return Err("AIL-EXPR: invalid dotted field key".into());
                }
                ParsedExpr::leaf(Self::Field(field.into()))
            } else if let Some(op) = IntOperator::from_token(token) {
                let right = stack
                    .pop()
                    .ok_or("AIL-EXPR: operator lacks right operand")?;
                let left = stack.pop().ok_or("AIL-EXPR: operator lacks left operand")?;
                ParsedExpr::binary(op, left, right)?
            } else {
                return Err("AIL-EXPR: invalid postfix token".into());
            };
            stack.push(parsed);
        }
        if stack.len() != 1 {
            return Err("AIL-EXPR: postfix expression must produce exactly one value".into());
        }
        Ok(stack.pop().unwrap().expression)
    }

    pub fn to_postfix(&self) -> String {
        let mut tokens = Vec::new();
        let mut pending = vec![(self, false)];
        while let Some((expression, visited)) = pending.pop() {
            match expression {
                Self::Literal(value) => tokens.push(format!("i:{value}")),
                Self::Field(key) => tokens.push(format!("f:{key}")),
                Self::Binary { op, .. } if visited => tokens.push(op.token().into()),
                Self::Binary { left, right, .. } => {
                    pending.push((expression, true));
                    pending.push((right, false));
                    pending.push((left, false));
                }
            }
        }
        tokens.join(" ")
    }

    pub fn to_source(&self) -> String {
        enum Part<'a> {
            Expression(&'a IntExpr),
            Text(&'static str),
        }
        let mut source = String::new();
        let mut pending = vec![Part::Expression(self)];
        while let Some(part) = pending.pop() {
            match part {
                Part::Text(text) => source.push_str(text),
                Part::Expression(Self::Literal(value)) => source.push_str(&value.to_string()),
                Part::Expression(Self::Field(key)) => source.push_str(key),
                Part::Expression(Self::Binary { op, left, right }) => {
                    pending.push(Part::Text(")"));
                    pending.push(Part::Expression(right));
                    pending.push(Part::Text(" "));
                    pending.push(Part::Text(op.token()));
                    pending.push(Part::Text(" "));
                    pending.push(Part::Expression(left));
                    pending.push(Part::Text("("));
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
                Self::Literal(_) => {}
            }
        }
        fields
    }

    /// Evaluate an expression whose field types have been checked as Int by Core.
    /// The map is a transport boundary; intermediate arithmetic values are typed.
    pub fn eval(&self, state: &BTreeMap<String, String>) -> Result<i64, String> {
        self.eval_borrowed(state)
    }
    pub fn eval_borrowed<V: AsRef<str>>(&self, state: &BTreeMap<String, V>) -> Result<i64, String> {
        let mut pending = vec![(self, false, 1)];
        let mut values: Vec<Value> = Vec::new();
        let mut nodes = 0;
        while let Some((expression, visited, depth)) = pending.pop() {
            if !visited {
                nodes += 1;
                if nodes > MAX_EXPRESSION_NODES || depth > MAX_EXPRESSION_DEPTH {
                    return Err("AIL-RUNTIME-INT: expression exceeds size or depth limit".into());
                }
            }
            match expression {
                Self::Literal(value) => values.push(Value::Int(*value)),
                Self::Field(key) => {
                    let text = state
                        .get(key)
                        .ok_or_else(|| format!("AIL-RUNTIME-INT: missing integer field {key}"))?;
                    let value = Value::Text(text.as_ref().to_owned())
                        .decode_declared_int()
                        .map_err(|error| format!("{error} in field {key}"))?;
                    values.push(value);
                }
                Self::Binary { op, .. } if visited => {
                    let right = values
                        .pop()
                        .ok_or("AIL-RUNTIME-INT: missing right operand")?
                        .require_int()?;
                    let left = values
                        .pop()
                        .ok_or("AIL-RUNTIME-INT: missing left operand")?
                        .require_int()?;
                    if matches!(op, IntOperator::Divide | IntOperator::Remainder) && right == 0 {
                        return Err(
                            "AIL-RUNTIME-DIVZERO: integer division or remainder by zero".into()
                        );
                    }
                    let result = match op {
                        IntOperator::Add => left.checked_add(right),
                        IntOperator::Subtract => left.checked_sub(right),
                        IntOperator::Multiply => left.checked_mul(right),
                        IntOperator::Divide => left.checked_div(right),
                        IntOperator::Remainder => left.checked_rem(right),
                    }
                    .ok_or("AIL-RUNTIME-OVERFLOW: signed 64-bit arithmetic overflow")?;
                    values.push(Value::Int(result));
                }
                Self::Binary { left, right, .. } => {
                    pending.push((expression, true, depth));
                    pending.push((right, false, depth + 1));
                    pending.push((left, false, depth + 1));
                }
            }
        }
        values
            .pop()
            .ok_or_else(|| "AIL-RUNTIME-INT: empty expression".to_string())?
            .require_int()
    }
}

struct ParsedExpr {
    expression: IntExpr,
    depth: usize,
}

impl ParsedExpr {
    fn leaf(expression: IntExpr) -> Self {
        Self {
            expression,
            depth: 1,
        }
    }

    fn binary(op: IntOperator, left: Self, right: Self) -> Result<Self, String> {
        let depth = left.depth.max(right.depth) + 1;
        if depth > MAX_EXPRESSION_DEPTH {
            return Err("AIL-EXPR: expression nesting is too deep".into());
        }
        Ok(Self {
            expression: IntExpr::Binary {
                op,
                left: Box::new(left.expression),
                right: Box::new(right.expression),
            },
            depth,
        })
    }
}

fn check_source_size(source: &str) -> Result<(), String> {
    if source.len() > MAX_EXPRESSION_BYTES {
        Err("AIL-EXPR: expression input is too large".into())
    } else {
        Ok(())
    }
}

fn is_field_key(key: &str) -> bool {
    key.contains('.')
        && key.split('.').all(|segment| {
            let mut bytes = segment.bytes();
            bytes
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
}

struct InfixParser<'a> {
    source: &'a str,
    position: usize,
    nodes: usize,
}

impl InfixParser<'_> {
    fn expression(&mut self, minimum_precedence: u8, nesting: usize) -> Result<ParsedExpr, String> {
        let mut left = self.atom(nesting)?;
        loop {
            self.skip_whitespace();
            let Some(byte) = self.peek() else { break };
            let (op, precedence) = match byte {
                b'+' => (IntOperator::Add, 1),
                b'-' => (IntOperator::Subtract, 1),
                b'*' => (IntOperator::Multiply, 2),
                b'/' => (IntOperator::Divide, 2),
                b'%' => (IntOperator::Remainder, 2),
                _ => break,
            };
            if precedence < minimum_precedence {
                break;
            }
            self.position += 1;
            let right = self.expression(precedence + 1, nesting)?;
            self.add_node()?;
            left = ParsedExpr::binary(op, left, right)?;
        }
        Ok(left)
    }

    fn atom(&mut self, nesting: usize) -> Result<ParsedExpr, String> {
        if nesting >= MAX_EXPRESSION_DEPTH {
            return Err("AIL-EXPR: expression nesting is too deep".into());
        }
        self.skip_whitespace();
        match self.peek() {
            Some(b'(') => {
                self.position += 1;
                let expression = self.expression(0, nesting + 1)?;
                self.skip_whitespace();
                if self.peek() != Some(b')') {
                    return Err("AIL-EXPR: expected closing parenthesis".into());
                }
                self.position += 1;
                Ok(expression)
            }
            Some(b'-') => {
                self.position += 1;
                self.skip_whitespace();
                if self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                    // Parse the sign together with the digits so MIN is a valid literal.
                    self.integer_literal(true)
                } else {
                    let right = self.atom(nesting + 1)?;
                    self.add_node()?;
                    self.add_node()?;
                    ParsedExpr::binary(
                        IntOperator::Subtract,
                        ParsedExpr::leaf(IntExpr::Literal(0)),
                        right,
                    )
                }
            }
            Some(byte) if byte.is_ascii_digit() => self.integer_literal(false),
            Some(byte) if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = self.position;
                while self.peek().is_some_and(|byte| {
                    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.'
                }) {
                    self.position += 1;
                }
                let key = &self.source[start..self.position];
                if !is_field_key(key) {
                    return Err("AIL-EXPR: expected a dotted field key".into());
                }
                self.add_node()?;
                Ok(ParsedExpr::leaf(IntExpr::Field(key.into())))
            }
            _ => Err("AIL-EXPR: expected an integer, field, unary minus, or parenthesis".into()),
        }
    }

    fn integer_literal(&mut self, negative: bool) -> Result<ParsedExpr, String> {
        let start = self.position;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.position += 1;
        }
        let digits = &self.source[start..self.position];
        let value = if negative {
            parse_integer(&format!("-{digits}"))?
        } else {
            parse_integer(digits)?
        };
        self.add_node()?;
        Ok(ParsedExpr::leaf(IntExpr::Literal(value)))
    }

    fn add_node(&mut self) -> Result<(), String> {
        self.nodes += 1;
        if self.nodes > MAX_EXPRESSION_NODES {
            return Err("AIL-EXPR: expression has too many nodes".into());
        }
        Ok(())
    }

    fn peek(&self) -> Option<u8> {
        self.source.as_bytes().get(self.position).copied()
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.position += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_transport_accepts_signed_decimal_and_full_i64_range() {
        for (source, expected) in [
            ("0", 0),
            ("-0", 0),
            ("007", 7),
            ("+7", 7),
            ("-007", -7),
            ("9223372036854775807", i64::MAX),
            ("-9223372036854775808", i64::MIN),
        ] {
            assert_eq!(parse_integer(source).unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn integer_transport_rejects_junk_and_out_of_range_values() {
        for source in [
            "",
            "+",
            "-",
            " 7",
            "7 ",
            "7\n",
            "7x",
            "1.0",
            "0x10",
            "1_000",
            "++7",
            "--7",
            "−7",
            "７",
            "9223372036854775808",
            "-9223372036854775809",
        ] {
            let error = parse_integer(source).unwrap_err();
            assert!(error.contains("AIL-RUNTIME-INT"), "{source:?}: {error}");
        }
    }

    #[test]
    fn postfix_preserves_operator_order_and_canonicalizes_literals() {
        let expression = IntExpr::from_postfix("f:sum.a i:+007 + i:-0 i:2 * -").unwrap();
        assert_eq!(expression.to_postfix(), "f:sum.a i:7 + i:0 i:2 * -");
        assert_eq!(expression.to_source(), "((sum.a + 7) - (0 * 2))");
        assert_eq!(expression.fields(), BTreeSet::from(["sum.a".into()]));
    }

    #[test]
    fn postfix_rejects_malformed_tokens_and_invalid_stack_shapes() {
        for source in [
            "",
            "+",
            "i:1 +",
            "i:1 i:2",
            "i:1 i:2 + +",
            "1",
            "i:",
            "i:1x",
            "i:9223372036854775808",
            "f:",
            "f:sum",
            "f:.sum.a",
            "f:sum..a",
            "f:sum.a.",
            "f:1sum.a",
            "f:sum.1a",
            "f:sum.a!",
            "f:sum.á",
            "f:sum.a i:1 **",
            "i:1i:2+",
        ] {
            assert!(IntExpr::from_postfix(source).is_err(), "{source:?}");
        }
    }

    #[test]
    fn postfix_and_source_preserve_nested_ast_structure() {
        let expression = IntExpr::from_postfix("f:z._first i:3 f:sum.a + * f:z._first -").unwrap();
        assert_eq!(
            expression.to_source(),
            "((z._first * (3 + sum.a)) - z._first)"
        );
        assert_eq!(
            expression.fields(),
            BTreeSet::from(["sum.a".into(), "z._first".into()])
        );
        assert_eq!(
            IntExpr::from_postfix(&expression.to_postfix()).unwrap(),
            expression
        );
    }

    #[test]
    fn postfix_rejects_oversized_inputs_and_excessive_tree_depth() {
        assert!(IntExpr::from_postfix(&" ".repeat(100_000)).is_err());
        let deep = format!("i:0 {}", "i:1 + ".repeat(1_000));
        assert!(IntExpr::from_postfix(&deep).is_err());
        let too_many_nodes = "i:0 ".repeat(10_000);
        assert!(IntExpr::from_postfix(&too_many_nodes).is_err());
    }

    #[test]
    fn infix_respects_precedence_parentheses_and_left_associativity() {
        for (source, expected) in [
            ("2 + 3 * 4", "i:2 i:3 i:4 * +"),
            ("(2 + 3) * 4", "i:2 i:3 + i:4 *"),
            ("20 / 5 * 2 % 3", "i:20 i:5 / i:2 * i:3 %"),
            ("10 - 3 - 2", "i:10 i:3 - i:2 -"),
            ("\t sum.a +\n sum.b * 007 ", "f:sum.a f:sum.b i:7 * +"),
            ("sum.a-(sum.b-sum.c)", "f:sum.a f:sum.b f:sum.c - -"),
        ] {
            assert_eq!(
                IntExpr::parse(source).unwrap().to_postfix(),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn infix_supports_unary_minus_and_minimum_integer() {
        for (source, expected) in [
            ("-9223372036854775808", "i:-9223372036854775808"),
            ("- 9223372036854775808", "i:-9223372036854775808"),
            ("-007", "i:-7"),
            ("-sum.a * 2", "i:0 f:sum.a - i:2 *"),
            ("-(2 + 3)", "i:0 i:2 i:3 + -"),
            ("--7", "i:0 i:-7 -"),
            ("2--3", "i:2 i:-3 -"),
        ] {
            assert_eq!(
                IntExpr::parse(source).unwrap().to_postfix(),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn infix_rejects_invalid_syntax_and_out_of_range_literals() {
        for source in [
            "",
            " ",
            "()",
            "(1",
            "1)",
            "1 2",
            "1 +",
            "* 2",
            "sum.a sum.b",
            "1 ** 2",
            "1 // 2",
            "sum",
            "sum..a",
            "sum.a.",
            ".sum.a",
            "sum.1a",
            "1.0",
            "0x10",
            "1_000",
            "á.a",
            "sum.a!",
            "sum . a",
            "'7'",
            "+7",
            "9223372036854775808",
            "-9223372036854775809",
            "-(9223372036854775808)",
        ] {
            assert!(IntExpr::parse(source).is_err(), "{source:?}");
        }
    }

    #[test]
    fn source_serialization_round_trips_the_ast() {
        for source in [
            "-9223372036854775808",
            "sum.a * (sum.b + 3) - -sum.c",
            "-(7 % 2) / -3",
        ] {
            let expression = IntExpr::parse(source).unwrap();
            assert_eq!(IntExpr::parse(&expression.to_source()).unwrap(), expression);
        }
    }

    #[test]
    fn infix_rejects_deep_parentheses_unary_chains_and_long_binary_chains() {
        assert!(IntExpr::parse(&" ".repeat(100_000)).is_err());
        assert!(IntExpr::parse(&format!("{}0{}", "(".repeat(1_000), ")".repeat(1_000))).is_err());
        assert!(IntExpr::parse(&format!("{}0", "-".repeat(1_000))).is_err());
        assert!(IntExpr::parse(&format!("0{}", "+1".repeat(1_000))).is_err());
        assert!(IntExpr::parse("((((sum.a + 1))))").is_ok());
    }

    #[test]
    fn evaluation_reads_fields_and_produces_integer_arithmetic() {
        let state = BTreeMap::from([
            ("sum.a".into(), "007".into()),
            ("sum.b".into(), "+3".into()),
            ("sum.c".into(), "-2".into()),
        ]);
        assert_eq!(
            IntExpr::parse("sum.a + sum.b * sum.c")
                .unwrap()
                .eval(&state)
                .unwrap(),
            1
        );
        assert_eq!(
            IntExpr::parse("-(sum.a + sum.b)")
                .unwrap()
                .eval(&state)
                .unwrap(),
            -10
        );
        assert_eq!(IntExpr::parse("sum.a").unwrap().eval(&state).unwrap(), 7);
        assert_eq!(
            IntExpr::parse("-9223372036854775808")
                .unwrap()
                .eval(&state)
                .unwrap(),
            i64::MIN
        );
    }

    #[test]
    fn division_truncates_toward_zero_and_remainder_follows_dividend_sign() {
        for (source, expected) in [
            ("7 / 3", 2),
            ("-7 / 3", -2),
            ("7 / -3", -2),
            ("-7 / -3", 2),
            ("7 % 3", 1),
            ("-7 % 3", -1),
            ("7 % -3", 1),
            ("-7 % -3", -1),
            ("-9223372036854775808 / 1", i64::MIN),
            ("-9223372036854775808 % 1", 0),
        ] {
            assert_eq!(
                IntExpr::parse(source)
                    .unwrap()
                    .eval(&BTreeMap::new())
                    .unwrap(),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn arithmetic_reports_overflow_instead_of_wrapping_or_panicking() {
        for source in [
            "9223372036854775807 + 1",
            "-9223372036854775808 - 1",
            "9223372036854775807 * 2",
            "-(-9223372036854775808)",
            "-9223372036854775808 / -1",
            "-9223372036854775808 % -1",
        ] {
            let error = IntExpr::parse(source)
                .unwrap()
                .eval(&BTreeMap::new())
                .unwrap_err();
            assert!(error.contains("AIL-RUNTIME-OVERFLOW"), "{source}: {error}");
        }
    }

    #[test]
    fn arithmetic_reports_division_and_remainder_by_zero() {
        for source in ["7 / 0", "7 % -0", "-9223372036854775808 / 0", "1 % (3 - 3)"] {
            let error = IntExpr::parse(source)
                .unwrap()
                .eval(&BTreeMap::new())
                .unwrap_err();
            assert!(error.contains("AIL-RUNTIME-DIVZERO"), "{source}: {error}");
        }
    }

    #[test]
    fn evaluation_rejects_missing_or_malformed_integer_fields() {
        let expression = IntExpr::parse("sum.a + 1").unwrap();
        assert!(
            expression
                .eval(&BTreeMap::new())
                .unwrap_err()
                .contains("AIL-RUNTIME-INT")
        );
        for value in ["hello", "3x", "", " 3", "3 ", "9223372036854775808"] {
            let state = BTreeMap::from([("sum.a".into(), value.into())]);
            assert!(
                expression
                    .eval(&state)
                    .unwrap_err()
                    .contains("AIL-RUNTIME-INT"),
                "{value:?}"
            );
        }
    }

    #[test]
    fn numeric_text_is_not_an_integer_operand() {
        assert_eq!(Value::Int(7).require_int().unwrap(), 7);
        assert!(
            Value::Text("7".into())
                .require_int()
                .unwrap_err()
                .contains("AIL-RUNTIME-INT")
        );
    }

    #[test]
    fn large_shallow_expressions_cannot_bypass_the_node_limit() {
        let mut expression = IntExpr::Literal(1);
        for _ in 0..9 {
            expression = IntExpr::Binary {
                op: IntOperator::Add,
                left: Box::new(expression.clone()),
                right: Box::new(expression),
            };
        }
        assert_eq!(expression.eval(&BTreeMap::new()).unwrap(), 512);
        assert_eq!(IntExpr::parse(&expression.to_source()).unwrap(), expression);
        assert_eq!(
            IntExpr::from_postfix(&expression.to_postfix()).unwrap(),
            expression
        );
        let oversized = IntExpr::Binary {
            op: IntOperator::Add,
            left: Box::new(expression.clone()),
            right: Box::new(expression),
        };
        assert!(IntExpr::parse(&oversized.to_source()).is_err());
        assert!(IntExpr::from_postfix(&oversized.to_postfix()).is_err());
        assert!(oversized.eval(&BTreeMap::new()).is_err());
    }

    #[test]
    fn deepest_accepted_tree_survives_both_serialization_round_trips() {
        let expression = IntExpr::parse(&format!("0{}", "+1".repeat(127))).unwrap();
        assert_eq!(expression.eval(&BTreeMap::new()).unwrap(), 127);
        assert_eq!(IntExpr::parse(&expression.to_source()).unwrap(), expression);
        assert_eq!(
            IntExpr::from_postfix(&expression.to_postfix()).unwrap(),
            expression
        );
        assert!(IntExpr::parse(&format!("0{}", "+1".repeat(128))).is_err());
    }
}
