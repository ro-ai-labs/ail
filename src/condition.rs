//! A single comparison between checked integer expressions.
//!
//! The expression limits apply to the whole condition: the input byte budget
//! includes both operands and the comparator (plus two separating spaces for
//! postfix input), and the comparison counts as one AST node at depth one.
//! Each operand also retains the expression parser's source nesting checks.
//! Evaluation checks the combined AST node/depth budgets, including conditions
//! constructed directly through their public fields.

use std::collections::{BTreeMap, BTreeSet};

use crate::expression::{
    IntExpr, MAX_EXPRESSION_BYTES, MAX_EXPRESSION_DEPTH, MAX_EXPRESSION_NODES,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntComparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl IntComparison {
    pub fn token(self) -> &'static str {
        match self {
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
        }
    }

    pub fn from_token(token: &str) -> Result<Self, String> {
        match token {
            "==" => Ok(Self::Equal),
            "!=" => Ok(Self::NotEqual),
            "<" => Ok(Self::Less),
            "<=" => Ok(Self::LessEqual),
            ">" => Ok(Self::Greater),
            ">=" => Ok(Self::GreaterEqual),
            _ => Err("AIL-CONDITION: expected ==, !=, <, <=, >, or >=".into()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntCondition {
    pub left: IntExpr,
    pub comparison: IntComparison,
    pub right: IntExpr,
}

impl IntCondition {
    pub fn parse(source: &str) -> Result<Self, String> {
        check_input_size(source.len())?;
        let is_comparison_byte = |byte: u8| matches!(byte, b'=' | b'!' | b'<' | b'>');
        let start = source
            .bytes()
            .position(is_comparison_byte)
            .ok_or("AIL-CONDITION: expected one integer comparison")?;
        let end = start
            + source.as_bytes()[start..]
                .iter()
                .take_while(|byte| is_comparison_byte(**byte))
                .count();
        let comparison = IntComparison::from_token(&source[start..end])?;
        if source[end..].bytes().any(is_comparison_byte) {
            return Err("AIL-CONDITION: comparison chains are not supported".into());
        }
        let condition = Self {
            left: IntExpr::parse(&source[..start])?,
            comparison,
            right: IntExpr::parse(&source[end..])?,
        };
        condition
            .check_structure()
            .map_err(|error| format!("AIL-CONDITION: {error}"))?;
        Ok(condition)
    }

    pub fn from_postfix(left: &str, comparison: &str, right: &str) -> Result<Self, String> {
        check_input_size(
            left.len()
                .saturating_add(comparison.len())
                .saturating_add(right.len())
                .saturating_add(2),
        )?;
        let comparison = IntComparison::from_token(comparison)?;
        let condition = Self {
            left: IntExpr::from_postfix(left)?,
            comparison,
            right: IntExpr::from_postfix(right)?,
        };
        condition
            .check_structure()
            .map_err(|error| format!("AIL-CONDITION: {error}"))?;
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

    /// Core must check that referenced fields are declared Int before execution.
    /// Comparing directly preserves the full signed range without subtraction.
    pub fn eval(&self, state: &BTreeMap<String, String>) -> Result<bool, String> {
        self.eval_borrowed(state)
    }
    pub fn eval_borrowed<V: AsRef<str>>(
        &self,
        state: &BTreeMap<String, V>,
    ) -> Result<bool, String> {
        self.check_structure()
            .map_err(|error| format!("AIL-RUNTIME-INT: {error}"))?;
        let left = self.left.eval_borrowed(state)?;
        let right = self.right.eval_borrowed(state)?;
        Ok(match self.comparison {
            IntComparison::Equal => left == right,
            IntComparison::NotEqual => left != right,
            IntComparison::Less => left < right,
            IntComparison::LessEqual => left <= right,
            IntComparison::Greater => left > right,
            IntComparison::GreaterEqual => left >= right,
        })
    }

    pub fn fields(&self) -> BTreeSet<String> {
        let mut fields = self.left.fields();
        fields.extend(self.right.fields());
        fields
    }

    fn check_structure(&self) -> Result<(), &'static str> {
        let mut nodes = 1;
        let mut pending = vec![(&self.right, 2), (&self.left, 2)];
        while let Some((expression, depth)) = pending.pop() {
            nodes += 1;
            if nodes > MAX_EXPRESSION_NODES {
                return Err("condition has too many nodes");
            }
            if depth > MAX_EXPRESSION_DEPTH {
                return Err("condition nesting is too deep");
            }
            if let IntExpr::Binary { left, right, .. } = expression {
                pending.push((right, depth + 1));
                pending.push((left, depth + 1));
            }
        }
        Ok(())
    }
}

fn check_input_size(bytes: usize) -> Result<(), String> {
    if bytes > MAX_EXPRESSION_BYTES {
        Err("AIL-CONDITION: condition input is too large".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::IntOperator;

    #[test]
    fn comparison_tokens_are_exact_and_round_trip() {
        for (token, comparison) in [
            ("==", IntComparison::Equal),
            ("!=", IntComparison::NotEqual),
            ("<", IntComparison::Less),
            ("<=", IntComparison::LessEqual),
            (">", IntComparison::Greater),
            (">=", IntComparison::GreaterEqual),
        ] {
            assert_eq!(IntComparison::from_token(token).unwrap(), comparison);
            assert_eq!(comparison.token(), token);
        }
        for token in ["", "=", "!", "===", "!==", "<>", "=<", "=>", " <", "<= "] {
            assert!(IntComparison::from_token(token).is_err(), "{token:?}");
        }
    }

    #[test]
    fn all_comparisons_evaluate_signed_values_including_equal_operands() {
        for (token, less, equal, greater) in [
            ("==", false, true, false),
            ("!=", true, false, true),
            ("<", true, false, false),
            ("<=", true, true, false),
            (">", false, false, true),
            (">=", false, true, true),
        ] {
            for (left, right, expected) in [(-3, 2, less), (-3, -3, equal), (2, -3, greater)] {
                let source = format!("{left}{token}{right}");
                assert_eq!(
                    IntCondition::parse(&source)
                        .unwrap()
                        .eval(&BTreeMap::new())
                        .unwrap(),
                    expected,
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn comparisons_do_not_subtract_signed_boundary_operands() {
        for source in [
            "-9223372036854775808 < 9223372036854775807",
            "9223372036854775807 > -9223372036854775808",
            "-9223372036854775808 != 9223372036854775807",
            "-9223372036854775808 <= -9223372036854775808",
            "9223372036854775807 >= 9223372036854775807",
            "-9223372036854775808 == -9223372036854775808",
        ] {
            assert!(
                IntCondition::parse(source)
                    .unwrap()
                    .eval(&BTreeMap::new())
                    .unwrap(),
                "{source}"
            );
        }
    }

    #[test]
    fn comparison_operands_use_full_checked_integer_expressions() {
        let state = BTreeMap::from([
            ("sum.a".into(), "007".into()),
            ("sum.b".into(), "+3".into()),
            ("sum.c".into(), "-2".into()),
        ]);
        for source in [
            "sum.a + sum.b * sum.c == 1",
            "(sum.a + sum.b) * sum.c < -(sum.a + 2)",
            "20 / 5 * 2 % 3 == 10 - 3 - 5",
            "\t -sum.c\n>= sum.b - 1 ",
        ] {
            assert!(
                IntCondition::parse(source).unwrap().eval(&state).unwrap(),
                "{source}"
            );
        }
    }

    #[test]
    fn source_and_postfix_reconstruction_preserve_conditions() {
        let condition =
            IntCondition::from_postfix("f:sum.a i:+007 +", "<=", "i:2 f:sum.b *").unwrap();
        assert_eq!(condition.to_source(), "(sum.a + 7) <= (2 * sum.b)");
        assert_eq!(
            IntCondition::parse(&condition.to_source()).unwrap(),
            condition
        );
        assert_eq!(
            condition.fields(),
            BTreeSet::from(["sum.a".into(), "sum.b".into()])
        );
        assert_eq!(
            IntCondition::parse("sum.a + sum.a != sum.a - sum.b")
                .unwrap()
                .fields(),
            BTreeSet::from(["sum.a".into(), "sum.b".into()])
        );
    }

    #[test]
    fn infix_rejects_missing_chained_and_ambiguous_comparisons() {
        for source in [
            "",
            " ",
            "1",
            "1 + 2",
            "1 ==",
            "<= 2",
            "1 = 2",
            "1 ! 2",
            "1 === 2",
            "1 !== 2",
            "1 <> 2",
            "1 =< 2",
            "1 => 2",
            "1 << 2",
            "1 >> 2",
            "1 < = 2",
            "1 < 2 < 3",
            "1 == 2 != 3",
            "1 < (2 >= 3)",
            "(1 < 2) == 1",
            "(1 < 2)",
            "1 == 1 && 2 == 2",
            "1 == 1 or 0",
            "1 < 2;",
            "1 < 2:",
        ] {
            assert!(IntCondition::parse(source).is_err(), "{source:?}");
        }
    }

    #[test]
    fn infix_rejects_noninteger_literals_and_invalid_expressions() {
        for source in [
            "'7' == 7",
            "\"7\" == 7",
            "true == 1",
            "1 == false",
            "1.5 < 2",
            "1 < 2.5",
            "1 < 0x10",
            "1 < 1_000",
            "1 < ７",
            "1 < +7",
            "1 < 9223372036854775808",
            "-9223372036854775809 < 1",
            "sum == 1",
            "sum..a == 1",
            "(1 + 2 < 3",
            "1 + < 2",
            "1 == (2))",
        ] {
            assert!(IntCondition::parse(source).is_err(), "{source:?}");
        }
    }

    #[test]
    fn postfix_requires_valid_operands_and_one_exact_comparator() {
        for (left, comparison, right) in [
            ("", "==", "i:1"),
            ("i:1", "==", ""),
            ("i:1 i:2", "==", "i:3"),
            ("i:1", "==", "i:2 +"),
            ("f:sum", "==", "i:1"),
            ("i:1", "==", "i:1x"),
            ("i:1", "<>", "i:2"),
            ("i:1", "== ", "i:1"),
            ("1", "==", "i:1"),
        ] {
            assert!(
                IntCondition::from_postfix(left, comparison, right).is_err(),
                "{left:?} {comparison:?} {right:?}"
            );
        }
    }

    #[test]
    fn evaluation_reports_missing_and_malformed_fields_from_either_operand() {
        for source in ["sum.a == 1", "1 == sum.a"] {
            let condition = IntCondition::parse(source).unwrap();
            let error = condition.eval(&BTreeMap::new()).unwrap_err();
            assert!(
                error.contains("AIL-RUNTIME-INT") && error.contains("sum.a"),
                "{error}"
            );
            for value in ["", "7x", " 7", "7 ", "7.0", "true", "9223372036854775808"] {
                let state = BTreeMap::from([("sum.a".into(), value.into())]);
                let error = condition.eval(&state).unwrap_err();
                assert!(
                    error.contains("AIL-RUNTIME-INT") && error.contains("sum.a"),
                    "{value:?}: {error}"
                );
            }
        }
    }

    #[test]
    fn arithmetic_failures_propagate_from_both_operands() {
        for (source, expected_code) in [
            ("9223372036854775807 + 1 > 0", "AIL-RUNTIME-OVERFLOW"),
            ("0 < -9223372036854775808 - 1", "AIL-RUNTIME-OVERFLOW"),
            ("-9223372036854775808 / -1 == 0", "AIL-RUNTIME-OVERFLOW"),
            ("1 / 0 == 0", "AIL-RUNTIME-DIVZERO"),
            ("0 == 1 % 0", "AIL-RUNTIME-DIVZERO"),
        ] {
            let error = IntCondition::parse(source)
                .unwrap()
                .eval(&BTreeMap::new())
                .unwrap_err();
            assert!(error.contains(expected_code), "{source}: {error}");
        }
    }

    fn balanced_sum(levels: usize) -> IntExpr {
        let mut expression = IntExpr::Literal(1);
        for _ in 0..levels {
            expression = IntExpr::Binary {
                op: IntOperator::Add,
                left: Box::new(expression.clone()),
                right: Box::new(expression),
            };
        }
        expression
    }

    #[test]
    fn byte_budget_is_shared_across_operands_and_comparator() {
        let exact = format!("{}1==1", " ".repeat(MAX_EXPRESSION_BYTES - 4));
        assert!(IntCondition::parse(&exact).is_ok());
        assert!(IntCondition::parse(&format!(" {exact}")).is_err());
        let left = format!("{}i:1", " ".repeat(MAX_EXPRESSION_BYTES / 2 - 3));
        let right = format!("i:1{}", " ".repeat(MAX_EXPRESSION_BYTES / 2 - 7));
        assert!(IntCondition::from_postfix(&left, "==", &right).is_ok());
        assert!(IntCondition::from_postfix(&left, "==", &format!("{right} ")).is_err());
    }

    #[test]
    fn node_budget_counts_both_operands_and_the_comparison() {
        let expression = balanced_sum(8);
        let mut condition = IntCondition {
            left: expression.clone(),
            comparison: IntComparison::Equal,
            right: expression,
        };
        assert!(condition.eval(&BTreeMap::new()).unwrap());
        assert_eq!(
            IntCondition::parse(&condition.to_source()).unwrap(),
            condition
        );
        assert!(
            IntCondition::from_postfix(
                &condition.left.to_postfix(),
                "==",
                &condition.right.to_postfix()
            )
            .is_ok()
        );
        condition.left = IntExpr::Binary {
            op: IntOperator::Add,
            left: Box::new(condition.left),
            right: Box::new(IntExpr::Literal(0)),
        };
        assert!(IntCondition::parse(&condition.to_source()).is_err());
        assert!(
            IntCondition::from_postfix(
                &condition.left.to_postfix(),
                "==",
                &condition.right.to_postfix()
            )
            .is_err()
        );
        assert!(
            condition
                .eval(&BTreeMap::new())
                .unwrap_err()
                .contains("AIL-RUNTIME-INT")
        );
    }

    #[test]
    fn depth_budget_counts_the_comparison_and_rejects_nested_source() {
        let exact = format!("0{} == 0", "+0".repeat(MAX_EXPRESSION_DEPTH - 2));
        let condition = IntCondition::parse(&exact).unwrap();
        assert!(condition.eval(&BTreeMap::new()).unwrap());
        assert_eq!(
            IntCondition::parse(&condition.to_source()).unwrap(),
            condition
        );
        let too_deep = format!("0{} == 0", "+0".repeat(MAX_EXPRESSION_DEPTH - 1));
        assert!(IntCondition::parse(&too_deep).is_err());
        let left = IntExpr::parse(&format!("0{}", "+0".repeat(MAX_EXPRESSION_DEPTH - 1))).unwrap();
        assert!(IntCondition::from_postfix(&left.to_postfix(), "==", "i:0").is_err());
        let condition = IntCondition {
            left,
            comparison: IntComparison::Equal,
            right: IntExpr::Literal(0),
        };
        assert!(condition.eval(&BTreeMap::new()).is_err());
        let nested = format!(
            "{}0{} == 0",
            "(".repeat(MAX_EXPRESSION_DEPTH),
            ")".repeat(MAX_EXPRESSION_DEPTH)
        );
        assert!(IntCondition::parse(&nested).is_err());
    }
}
