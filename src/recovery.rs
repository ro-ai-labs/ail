//! Verified lexical recovery scopes shared by the VM and native emitter.
use super::*;
use crate::expression::IntExpr;

pub(super) const CATEGORIES: [&str; 15] = [
    "INT",
    "OVERFLOW",
    "DIVZERO",
    "FLOAT",
    "BOOL",
    "TEXT",
    "LIST",
    "INDEX",
    "JSON",
    "JSON-PATH",
    "PROCESS",
    "TIMEOUT",
    "PATH",
    "BYTES",
    "VALUE",
];
const CODES: [&str; 15] = [
    "AIL-RUNTIME-INT",
    "AIL-RUNTIME-OVERFLOW",
    "AIL-RUNTIME-DIVZERO",
    "AIL-RUNTIME-FLOAT",
    "AIL-RUNTIME-BOOL",
    "AIL-RUNTIME-TEXT",
    "AIL-RUNTIME-LIST",
    "AIL-RUNTIME-INDEX",
    "AIL-RUNTIME-JSON",
    "AIL-RUNTIME-JSON-PATH",
    "AIL-RUNTIME-PROCESS",
    "AIL-RUNTIME-TIMEOUT",
    "AIL-RUNTIME-PATH",
    "AIL-RUNTIME-BYTES",
    "AIL-RUNTIME-VALUE",
];

fn error(message: &str) -> String {
    format!("AILBC019: {message}")
}

pub(super) fn validate_raise(category: &str) -> Result<(), String> {
    if CATEGORIES.contains(&category) {
        Ok(())
    } else {
        Err(error("raise requires exactly one catchable category"))
    }
}
pub(super) fn verify_raise(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    if instruction.opcode != "RAISE"
        || instruction.operands.len() != 1
        || !instruction.operands.contains_key("category")
    {
        return Err(error("RAISE requires exactly category"));
    }
    validate_raise(&instruction.operands["category"])
}
pub(super) fn raised_error(category: &str) -> String {
    format!(
        "{}: explicitly raised error",
        code(category).expect("verified raise category")
    )
}

/// Preserve legacy procedures without RAISE. For raise-containing procedures,
/// follow normal and exceptional control flow and reject a reachable end PC.
pub(super) fn check_raise_termination(action: &AilBytecodeAction) -> Result<(), String> {
    if !action.instructions.iter().any(|i| i.opcode == "RAISE") {
        return Ok(());
    }
    let layout = Layout::analyze(action)?;
    let (labels, diagnostics) = ail_bytecode_action_labels(action);
    if !diagnostics.is_empty() {
        return Err(error("invalid function labels"));
    }
    let mut pending = vec![0usize];
    let mut seen = BTreeSet::new();
    while let Some(pc) = pending.pop() {
        if !seen.insert(pc) {
            continue;
        }
        let Some(i) = action.instructions.get(pc) else {
            return Err(error("raise-containing function has reachable fallthrough"));
        };
        let target = || {
            i.operands
                .get("label")
                .and_then(|name| labels.get(name))
                .copied()
                .ok_or_else(|| error("invalid function jump"))
        };
        match i.opcode.as_str() {
            "RAISE" => {
                verify_raise(i)?;
                if let Some(handler) = layout.matching(pc, &i.operands["category"]) {
                    pending.push(handler.recover + 1);
                }
                continue;
            }
            "RETURN_INT" | "RETURN_VALUE" => {}
            "JUMP" => {
                pending.push(target()?);
                continue;
            }
            "RECOVER" => {
                pending.push(
                    layout
                        .recovering(pc)
                        .ok_or_else(|| error("missing recovery scope"))?
                        .end
                        + 1,
                );
                continue;
            }
            "BRANCH_INT" | "BRANCH_FLOAT" | "BRANCH_BOOL" => {
                pending.push(target()?);
                pending.push(pc + 1);
            }
            _ => pending.push(pc + 1),
        }
        // Data operations, calls, condition evaluation and return validation may
        // enter any matching handler. Structural instructions cannot fail.
        if !matches!(
            i.opcode.as_str(),
            "INTEGER_FUNCTION"
                | "VALUE_FUNCTION"
                | "LABEL"
                | "TRY_BEGIN"
                | "TRY_END"
                | "EMIT_TRACE"
        ) {
            for category in CATEGORIES {
                if let Some(handler) = layout.matching(pc, category) {
                    pending.push(handler.recover + 1);
                }
            }
        }
    }
    Ok(())
}

pub(super) fn source_errors(text: &str) -> Result<String, String> {
    if text == "any" {
        return Ok(text.into());
    }
    let mut categories = BTreeSet::new();
    for category in text.split(" or ") {
        if !CATEGORIES.contains(&category) || !categories.insert(category) {
            return Err(error(
                "recovery requires distinct known data-error categories",
            ));
        }
    }
    Ok(categories.into_iter().collect::<Vec<_>>().join(";"))
}

pub(super) fn mask(errors: &str) -> Result<u16, String> {
    if errors == "any" {
        return Ok((1 << CATEGORIES.len()) - 1);
    }
    let mut result = 0;
    let mut previous = None;
    for name in errors.split(';') {
        let index = CATEGORIES
            .iter()
            .position(|category| *category == name)
            .ok_or_else(|| error("unknown recovery data-error category"))?;
        if previous.is_some_and(|previous| previous >= name) {
            return Err(error("recovery categories must be sorted and unique"));
        }
        result |= 1 << index;
        previous = Some(name);
    }
    Ok(result)
}

pub(super) fn category(message: &str) -> Option<&'static str> {
    let prefix = message.split_once(':')?.0;
    CODES
        .iter()
        .position(|code| *code == prefix)
        .map(|i| CATEGORIES[i])
}

pub(super) fn code(category: &str) -> Option<&'static str> {
    CATEGORIES
        .iter()
        .position(|name| *name == category)
        .map(|i| CODES[i])
}

pub(super) struct Handler {
    pub begin: usize,
    pub recover: usize,
    pub end: usize,
    pub mask: u16,
    pub key: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Region {
    handler: usize,
    recovery: bool,
}

pub(super) struct Layout {
    pub handlers: Vec<Handler>,
    regions: Vec<Vec<Region>>,
    protected: Vec<Vec<usize>>,
}

impl Layout {
    pub fn analyze(action: &AilBytecodeAction) -> Result<Self, String> {
        Self::analyze_with_depth_limit(action, Some(64))
    }

    /// Native action calls are expanded only after each original procedure has
    /// passed verification. Their combined handler chain can exceed one
    /// procedure's lexical depth; retain all pairing and jump-region checks.
    pub fn analyze_expanded(action: &AilBytecodeAction) -> Result<Self, String> {
        Self::analyze_with_depth_limit(action, None)
    }

    fn analyze_with_depth_limit(
        action: &AilBytecodeAction,
        depth_limit: Option<usize>,
    ) -> Result<Self, String> {
        let mut layout = Self {
            handlers: Vec::new(),
            regions: Vec::new(),
            protected: Vec::new(),
        };
        let mut stack: Vec<Region> = Vec::new();
        for (pc, instruction) in action.instructions.iter().enumerate() {
            layout.regions.push(stack.clone());
            layout.protected.push(
                stack
                    .iter()
                    .filter(|r| !r.recovery)
                    .map(|r| r.handler)
                    .collect(),
            );
            let required: &[&str] = match instruction.opcode.as_str() {
                "RAISE" => {
                    verify_raise(instruction)?;
                    continue;
                }
                "TRY_BEGIN" => &["errors", "key"],
                "RECOVER" | "TRY_END" => &[],
                _ => continue,
            };
            if instruction.operands.len() != required.len()
                || required
                    .iter()
                    .any(|name| !instruction.operands.contains_key(*name))
            {
                return Err(error("unexpected or missing recovery operand"));
            }
            match instruction.opcode.as_str() {
                "TRY_BEGIN" => {
                    if depth_limit.is_some_and(|limit| stack.len() >= limit) {
                        return Err(error("recovery nesting exceeds 64 scopes"));
                    }
                    let key = &instruction.operands["key"];
                    if !matches!(IntExpr::parse(key), Ok(IntExpr::Field(parsed)) if parsed == *key)
                    {
                        return Err(error("recovery destination must be a field reference"));
                    }
                    let index = layout.handlers.len();
                    layout.handlers.push(Handler {
                        begin: pc,
                        recover: usize::MAX,
                        end: usize::MAX,
                        mask: mask(&instruction.operands["errors"])?,
                        key: key.clone(),
                    });
                    stack.push(Region {
                        handler: index,
                        recovery: false,
                    });
                }
                "RECOVER" => {
                    let region = stack
                        .last_mut()
                        .filter(|r| !r.recovery)
                        .ok_or_else(|| error("RECOVER requires a protected scope"))?;
                    layout.handlers[region.handler].recover = pc;
                    region.recovery = true;
                }
                "TRY_END" => {
                    let region = stack
                        .pop()
                        .filter(|r| r.recovery)
                        .ok_or_else(|| error("TRY_END requires a recovery body"))?;
                    layout.handlers[region.handler].end = pc;
                }
                _ => unreachable!(),
            }
        }
        if !stack.is_empty() {
            return Err(error("unclosed recovery scope"));
        }
        layout.regions.push(Vec::new());
        layout.protected.push(Vec::new());
        if layout.handlers.is_empty() {
            return Ok(layout);
        }
        let (labels, _) = ail_bytecode_action_labels(action);
        for (from, instruction) in action.instructions.iter().enumerate() {
            if matches!(
                instruction.opcode.as_str(),
                "JUMP" | "BRANCH_INT" | "BRANCH_FLOAT" | "BRANCH_BOOL" | "BRANCH_FIELD_EQUALS"
            ) {
                let to = instruction
                    .operands
                    .get("label")
                    .and_then(|label| labels.get(label))
                    .ok_or_else(|| {
                        error("recovery control flow requires an existing target label")
                    })?;
                layout.exited(from, *to)?;
            }
        }
        Ok(layout)
    }

    pub fn protected(&self, pc: usize) -> &[usize] {
        self.protected.get(pc).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn beginning(&self, pc: usize) -> Option<&Handler> {
        self.handlers.iter().find(|handler| handler.begin == pc)
    }

    pub fn recovering(&self, pc: usize) -> Option<&Handler> {
        self.handlers.iter().find(|handler| handler.recover == pc)
    }

    pub fn exited(&self, from: usize, to: usize) -> Result<usize, String> {
        let source = self
            .regions
            .get(from)
            .ok_or_else(|| error("invalid recovery source PC"))?;
        let target = self
            .regions
            .get(to)
            .ok_or_else(|| error("invalid recovery target PC"))?;
        if !source.starts_with(target) {
            return Err(error("jump enters a protected or recovery region"));
        }
        Ok(self.protected(from).len() - self.protected(to).len())
    }

    pub fn matching(&self, pc: usize, category: &str) -> Option<&Handler> {
        let bit = 1u16 << CATEGORIES.iter().position(|name| *name == category)?;
        self.protected(pc)
            .iter()
            .rev()
            .map(|i| &self.handlers[*i])
            .find(|handler| handler.mask & bit != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_errors_match_exact_codes_without_catching_fatal_prefixes() {
        for name in CATEGORIES {
            assert_eq!(
                category(&format!("{}: detail", code(name).unwrap())),
                Some(name)
            );
            assert_eq!(category(&format!("AIL-RUNTIME-{name}-LIMIT: detail")), None);
        }
        for message in [
            "AIL-RUNTIME-IO: detail",
            "prefix AIL-RUNTIME-INT: detail",
            "AIL-RUNTIME-INT",
            "AIL bytecode execution step limit exceeded",
        ] {
            assert_eq!(category(message), None);
        }
    }

    #[test]
    fn source_filter_normalizes_only_valid_unique_names() {
        assert_eq!(
            source_errors("JSON-PATH or INT or JSON").unwrap(),
            "INT;JSON;JSON-PATH"
        );
        assert_eq!(source_errors("any").unwrap(), "any");
        assert_eq!(mask("any").unwrap(), 32767);
        assert_eq!(mask("VALUE").unwrap(), 16384);
        assert_eq!(mask("BYTES").unwrap(), 8192);
        assert_eq!(
            category("AIL-RUNTIME-VALUE: malformed variant"),
            Some("VALUE")
        );
        assert_eq!(category("AIL-RUNTIME-VALUE-LIMIT: too large"), None);
        assert_eq!(mask("PATH").unwrap(), 4096);
        assert_eq!(mask("INT").unwrap(), 1);
        assert_eq!(mask("PROCESS;TIMEOUT").unwrap(), 3072);
        for text in [
            "any or INT",
            "INT or INT",
            " INT",
            "INT ",
            "INT;JSON",
            "IO",
            "",
        ] {
            assert!(source_errors(text).is_err(), "{text}");
        }
    }
}
