//! Verified integer procedures with independent typed local frames.
use super::*;
use crate::condition::IntComparison;
use crate::expression::{IntExpr, IntOperator};

pub(super) fn is_function(action: &AilBytecodeAction) -> bool {
    action
        .instructions
        .first()
        .is_some_and(|i| i.opcode == "INTEGER_FUNCTION")
}
fn identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn parameters(action: &AilBytecodeAction) -> Result<Vec<String>, String> {
    let text = action
        .instructions
        .first()
        .and_then(|i| i.operands.get("parameters"))
        .ok_or("missing function parameters")?;
    let names = if text.is_empty() {
        Vec::new()
    } else {
        text.split(';').map(str::to_string).collect::<Vec<_>>()
    };
    if names.len() > 256
        || names.iter().any(|s| !identifier(s))
        || names.windows(2).any(|w| w[0] >= w[1])
    {
        return Err("function parameters must be unique sorted identifiers (maximum 256)".into());
    }
    Ok(names)
}
fn local(key: &str) -> bool {
    key.strip_prefix("local.").is_some_and(identifier)
}
fn field(key: &str) -> bool {
    matches!(IntExpr::parse(key),Ok(IntExpr::Field(parsed)) if parsed==key)
}

pub(super) fn check_source_call(
    call: &AilIntegerCall,
    document: &AilDocument,
) -> Result<(), String> {
    let target = document
        .functions
        .get(&call.target)
        .filter(|f| f.integer.is_some())
        .ok_or_else(|| {
            format!(
                "AIL-FUNCTION-001: unknown executable integer function {}",
                call.target
            )
        })?;
    if target.outputs.len() != 1
        || target.outputs.values().any(|o| o.type_name != "Int")
        || target.inputs.values().any(|i| i.type_name != "Int")
        || target.inputs.keys().ne(call.arguments.keys())
    {
        return Err("AIL-FUNCTION-001: integer call requires matching named Int arguments and one Int result".into());
    }
    for key in
        std::iter::once(call.key.clone()).chain(call.arguments.values().flat_map(IntExpr::fields))
    {
        if runtime_field_type_name(document, &key) != Some("Int") {
            return Err(format!(
                "AIL-FUNCTION-001: argument/destination {key} must be a declared Int field"
            ));
        }
    }
    Ok(())
}

pub(super) fn verify(program: &AilBytecodeProgram) -> Vec<String> {
    let mut errors = Vec::new();
    let functions = program
        .actions
        .iter()
        .filter(|(_, a)| is_function(a))
        .map(|(n, a)| (n, parameters(a)))
        .collect::<BTreeMap<_, _>>();
    for (name, action) in &program.actions {
        if is_function(action) && name != &action.name {
            errors.push(format!(
                "AILBC011 function map key {name} differs from procedure name {}",
                action.name
            ));
        }
    }
    for (name, params) in &functions {
        if let Err(e) = params {
            errors.push(format!("AILBC011 function {name}: {e}"));
        }
    }
    for action in program.actions.values() {
        let function = is_function(action);
        if function && let Err(e) = recovery::check_raise_termination(action) {
            errors.push(format!("AILBC011 function {}: {e}", action.name));
        }
        let mut locals = BTreeSet::new();
        if function && let Ok(names) = parameters(action) {
            locals.extend(names.into_iter().map(|n| format!("local.{n}")));
        }
        for (index, i) in action.instructions.iter().enumerate() {
            let mut error = None;
            if (function || i.opcode == "CALL_INT")
                && ail_bytecode_required_operands(&i.opcode)
                    .is_some_and(|keys| i.operands.keys().any(|k| !keys.contains(&k.as_str())))
            {
                error = Some("unexpected operand in integer function protocol".into());
            }
            let mut expressions = Vec::new();
            let mut keys = Vec::new();
            match i.opcode.as_str() {
                "INTEGER_FUNCTION" if index != 0 => {
                    error = Some("function signature must be first".to_string())
                }
                "CALL_INT" => {
                    keys.push(ail_bytecode_operand(i, "key"));
                    match integer_function::decode_arguments(ail_bytecode_operand(i, "arguments")) {
                        Ok(args) => {
                            if !functions
                                .get(&ail_bytecode_operand(i, "target").to_string())
                                .is_some_and(|p| {
                                    p.as_ref().is_ok_and(|names| names.iter().eq(args.keys()))
                                })
                            {
                                error=Some("integer call target or named arguments do not match an executable function".into());
                            }
                            expressions.extend(args.into_values());
                        }
                        Err(e) => error = Some(e),
                    }
                }
                "RETURN_INT" | "EVAL_INT" => {
                    if i.opcode == "RETURN_INT" && !function {
                        error = Some("RETURN_INT requires an integer function".into());
                    }
                    if i.opcode == "EVAL_INT" {
                        keys.push(ail_bytecode_operand(i, "key"));
                    }
                    match IntExpr::from_postfix(ail_bytecode_operand(i, "expression")) {
                        Ok(e) => expressions.push(e),
                        Err(e) => error = Some(e),
                    }
                }
                "BRANCH_INT" => {
                    for key in ["left", "right"] {
                        match IntExpr::from_postfix(ail_bytecode_operand(i, key)) {
                            Ok(e) => expressions.push(e),
                            Err(e) => error = Some(e),
                        }
                    }
                }
                "CALL_ACTION" | "REPEAT_ACTION"
                    if functions.contains_key(&ail_bytecode_operand(i, "target").to_string()) =>
                {
                    error = Some("integer functions require CALL_INT".into())
                }
                _ => {}
            }
            if keys.iter().any(|k| !field(k)) {
                error = Some("invalid integer destination field".into());
            }
            if function {
                if !matches!(
                    i.opcode.as_str(),
                    "INTEGER_FUNCTION"
                        | "CALL_INT"
                        | "RETURN_INT"
                        | "EVAL_INT"
                        | "BRANCH_INT"
                        | "LABEL"
                        | "JUMP"
                        | "EMIT_TRACE"
                        | "RAISE"
                ) {
                    error = Some(format!(
                        "{} is not permitted in an integer function",
                        i.opcode
                    ));
                }
                locals.extend(keys.into_iter().map(str::to_string));
                locals.extend(expressions.iter().flat_map(IntExpr::fields));
                if locals.iter().any(|k| !local(k)) || locals.len() > 256 {
                    error = Some("integer functions require at most 256 local fields".into());
                }
            }
            if let Some(error) = error {
                errors.push(format!(
                    "AILBC011 procedure {} instruction {index}: {error}",
                    action.name
                ));
            }
        }
    }
    errors
}

enum Expr {
    Literal(i64),
    Local(usize),
    Binary(IntOperator, Box<Expr>, Box<Expr>),
}
impl Expr {
    fn compile(expr: IntExpr, slots: &BTreeMap<String, usize>) -> Self {
        match expr {
            IntExpr::Literal(n) => Self::Literal(n),
            IntExpr::Field(key) => Self::Local(slots[&key]),
            IntExpr::Binary { op, left, right } => Self::Binary(
                op,
                Box::new(Self::compile(*left, slots)),
                Box::new(Self::compile(*right, slots)),
            ),
        }
    }
    fn eval(&self, frame: &[Option<i64>]) -> Result<i64, String> {
        match self {
            Self::Literal(n) => Ok(*n),
            Self::Local(slot) => {
                frame[*slot].ok_or_else(|| "AIL-RUNTIME-INT: uninitialized local field".into())
            }
            Self::Binary(op, left, right) => {
                let left = left.eval(frame)?;
                let right = right.eval(frame)?;
                if matches!(op, IntOperator::Divide | IntOperator::Remainder) && right == 0 {
                    return Err("AIL-RUNTIME-DIVZERO: integer division or remainder by zero".into());
                }
                match op {
                    IntOperator::Add => left.checked_add(right),
                    IntOperator::Subtract => left.checked_sub(right),
                    IntOperator::Multiply => left.checked_mul(right),
                    IntOperator::Divide => left.checked_div(right),
                    IntOperator::Remainder => left.checked_rem(right),
                }
                .ok_or_else(|| "AIL-RUNTIME-OVERFLOW: signed 64-bit arithmetic overflow".into())
            }
        }
    }
}
enum Step {
    Nop,
    Raise(String),
    Eval(usize, Expr),
    Call(usize, usize, Vec<Expr>),
    Return(Expr),
    Branch(Expr, IntComparison, Expr, usize),
    Jump(usize),
    Trace(String),
}
struct Routine {
    parameters: Vec<usize>,
    slots: usize,
    steps: Vec<Step>,
}
pub(super) struct Machine {
    names: BTreeMap<String, usize>,
    routines: Vec<Routine>,
}
impl Machine {
    pub(super) fn new(program: &AilBytecodeProgram) -> Result<Self, String> {
        let functions = program
            .actions
            .values()
            .filter(|a| is_function(a))
            .collect::<Vec<_>>();
        let names = functions
            .iter()
            .enumerate()
            .map(|(i, a)| (a.name.clone(), i))
            .collect::<BTreeMap<_, _>>();
        let mut routines = Vec::new();
        for function in functions {
            let params = parameters(function)?;
            let mut fields = params
                .iter()
                .map(|n| format!("local.{n}"))
                .collect::<BTreeSet<_>>();
            let mut exprs = BTreeMap::new();
            let mut calls = BTreeMap::new();
            for (index, i) in function.instructions.iter().enumerate() {
                if matches!(i.opcode.as_str(), "EVAL_INT" | "CALL_INT")
                    && let Some(key) = i.operands.get("key")
                {
                    fields.insert(key.clone());
                }
                let keys: &[&str] = match i.opcode.as_str() {
                    "EVAL_INT" | "RETURN_INT" => &["expression"],
                    "BRANCH_INT" => &["left", "right"],
                    _ => &[],
                };
                for key in keys {
                    let expr = IntExpr::from_postfix(ail_bytecode_operand(i, key))?;
                    fields.extend(expr.fields());
                    exprs.insert((index, *key), expr);
                }
                if i.opcode == "CALL_INT" {
                    let args =
                        integer_function::decode_arguments(ail_bytecode_operand(i, "arguments"))?;
                    for e in args.values() {
                        fields.extend(e.fields());
                    }
                    calls.insert(index, args);
                }
            }
            let slots = fields
                .into_iter()
                .enumerate()
                .map(|(i, key)| (key, i))
                .collect::<BTreeMap<_, _>>();
            let (labels, _) = ail_bytecode_action_labels(function);
            let mut steps = Vec::new();
            for (index, i) in function.instructions.iter().enumerate() {
                let expr = |key| Expr::compile(exprs[&(index, key)].clone(), &slots);
                steps.push(match i.opcode.as_str() {
                    "INTEGER_FUNCTION" | "LABEL" => Step::Nop,
                    "RAISE" => Step::Raise(ail_bytecode_operand(i, "category").into()),
                    "EVAL_INT" => {
                        Step::Eval(slots[ail_bytecode_operand(i, "key")], expr("expression"))
                    }
                    "RETURN_INT" => Step::Return(expr("expression")),
                    "CALL_INT" => Step::Call(
                        slots[ail_bytecode_operand(i, "key")],
                        names[ail_bytecode_operand(i, "target")],
                        calls[&index]
                            .values()
                            .cloned()
                            .map(|e| Expr::compile(e, &slots))
                            .collect(),
                    ),
                    "JUMP" => Step::Jump(labels[ail_bytecode_operand(i, "label")]),
                    "BRANCH_INT" => Step::Branch(
                        expr("left"),
                        IntComparison::from_token(ail_bytecode_operand(i, "comparison"))?,
                        expr("right"),
                        labels[ail_bytecode_operand(i, "label")],
                    ),
                    "EMIT_TRACE" => Step::Trace(ail_bytecode_operand(i, "event").into()),
                    _ => {
                        return Err(format!(
                            "unsupported integer function instruction {}",
                            i.opcode
                        ));
                    }
                });
            }
            routines.push(Routine {
                parameters: params
                    .iter()
                    .map(|p| slots[&format!("local.{p}")])
                    .collect(),
                slots: slots.len(),
                steps,
            });
        }
        Ok(Self { names, routines })
    }
    pub(super) fn call(
        &self,
        name: &str,
        args: &[i64],
        trace: &mut Vec<String>,
    ) -> Result<i64, String> {
        self.call_at_depth(name, args, 0, trace)
    }
    pub(super) fn call_at_depth(
        &self,
        name: &str,
        args: &[i64],
        depth: usize,
        trace: &mut Vec<String>,
    ) -> Result<i64, String> {
        self.run(
            *self
                .names
                .get(name)
                .ok_or_else(|| format!("unknown integer function {name}"))?,
            args,
            depth,
            trace,
        )
    }
    fn run(
        &self,
        index: usize,
        args: &[i64],
        depth: usize,
        trace: &mut Vec<String>,
    ) -> Result<i64, String> {
        if depth >= 64 {
            return Err("AIL-RUNTIME-DEPTH: integer function call depth exceeded".into());
        }
        let routine = &self.routines[index];
        if args.len() != routine.parameters.len() {
            return Err("AIL-FUNCTION-001: argument count mismatch".into());
        }
        let mut frame = vec![None; routine.slots];
        for (slot, value) in routine.parameters.iter().zip(args) {
            frame[*slot] = Some(*value);
        }
        let mut ip = 0;
        let mut fuel = routine.steps.len().saturating_mul(1024).max(1024);
        while let Some(step) = routine.steps.get(ip) {
            fuel = fuel
                .checked_sub(1)
                .ok_or("AIL bytecode execution step limit exceeded")?;
            match step {
                Step::Nop => {}
                Step::Raise(category) => return Err(recovery::raised_error(category)),
                Step::Eval(slot, expr) => frame[*slot] = Some(expr.eval(&frame)?),
                Step::Return(expr) => return expr.eval(&frame),
                Step::Jump(target) => {
                    ip = *target;
                    continue;
                }
                Step::Trace(event) => trace.push(format!("trace {event}")),
                Step::Call(slot, target, args) => {
                    let args = args
                        .iter()
                        .map(|e| e.eval(&frame))
                        .collect::<Result<Vec<_>, _>>()?;
                    let value = self.run(*target, &args, depth + 1, trace)?;
                    frame[*slot] = Some(value);
                }
                Step::Branch(left, op, right, target) => {
                    let left = left.eval(&frame)?;
                    let right = right.eval(&frame)?;
                    let take = match op {
                        IntComparison::Equal => left == right,
                        IntComparison::NotEqual => left != right,
                        IntComparison::Less => left < right,
                        IntComparison::LessEqual => left <= right,
                        IntComparison::Greater => left > right,
                        IntComparison::GreaterEqual => left >= right,
                    };
                    if take {
                        ip = *target;
                        continue;
                    }
                }
            }
            ip += 1;
        }
        Err("AIL-RUNTIME-RETURN: integer function ended without a return value".into())
    }
}
