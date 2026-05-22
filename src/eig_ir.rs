use std::collections::{BTreeMap, BTreeSet};

use crate::collections::{
    collection_path_value_with, collection_query_keys_with, delete_collection_path_with,
};
use crate::core_model::{json_string, json_value};
use crate::expression;
use crate::predicate;
use crate::rif_model::{FailureCase, InvocationTarget, RifDocument};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BranchMode {
    Primary,
    Else,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BranchSection {
    Primary,
    Else,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeProgram {
    pub intent: String,
    pub document: crate::rif_model::RifDocument,
    pub instructions: Vec<Instruction>,
    pub handlers: Vec<FailureHandler>,
}

impl BytecodeProgram {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"intent\":{},\"instructions\":[{}],\"handlers\":[{}]}}",
            json_string(&self.intent),
            self.instructions
                .iter()
                .map(Instruction::to_json)
                .collect::<Vec<_>>()
                .join(","),
            self.handlers
                .iter()
                .map(FailureHandler::to_json)
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub opcode: String,
    pub fields: BTreeMap<String, String>,
}

impl Instruction {
    pub fn new(opcode: impl Into<String>, fields: &[(&str, String)]) -> Self {
        Self {
            opcode: opcode.into(),
            fields: fields
                .iter()
                .map(|(key, value)| ((*key).to_string(), value.clone()))
                .collect(),
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"opcode\":{},\"fields\":{}}}",
            json_string(&self.opcode),
            string_map_json(&self.fields)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureHandler {
    pub condition: String,
    pub actions: Vec<String>,
    pub stop_failure: Option<String>,
    pub ignored_failures: Vec<String>,
}

impl FailureHandler {
    fn from_rif(handler: &FailureCase) -> Self {
        Self {
            condition: handler.condition.clone(),
            actions: handler.actions.clone(),
            stop_failure: handler.stop_failure.clone(),
            ignored_failures: handler.ignored_failures.clone(),
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"condition\":{},\"actions\":[{}],\"stop_failure\":{},\"ignored_failures\":[{}]}}",
            json_string(&self.condition),
            self.actions
                .iter()
                .map(|action| json_string(action))
                .collect::<Vec<_>>()
                .join(","),
            self.stop_failure
                .as_ref()
                .map(|failure| json_string(failure))
                .unwrap_or_else(|| "null".to_string()),
            self.ignored_failures
                .iter()
                .map(|failure| json_string(failure))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeRunResult {
    pub status: String,
    pub final_state: BTreeMap<String, String>,
    pub outputs: BTreeMap<String, String>,
    pub trace: Vec<String>,
    pub failure: Option<String>,
}

const LOOP_LIMIT: usize = 1024;
const INVOKE_DEPTH_LIMIT: usize = 32;
type ParallelJoin = (
    BTreeMap<String, String>,
    BTreeMap<String, String>,
    Vec<String>,
);

pub fn lower_document(document: &RifDocument) -> BytecodeProgram {
    let mut instructions = Vec::new();
    for requirement in &document.intent.requires {
        instructions.push(Instruction::new(
            "CHECK_REQUIRES",
            &[("predicate", requirement.text.clone())],
        ));
    }

    for step in &document.intent.steps {
        let primary_contract = step
            .call
            .as_ref()
            .and_then(|call| document.application.operations.get(&call.target));
        let else_contract = step
            .otherwise_call
            .as_ref()
            .and_then(|call| document.application.operations.get(&call.target));
        instructions.push(Instruction::new(
            "BEGIN_STEP",
            &[
                ("step", step.title.clone()),
                ("guard", step.guard.clone().unwrap_or_default()),
                ("has_else", has_else_branch(step).to_string()),
            ],
        ));
        if let Some(query) = &step.iterate_over {
            instructions.push(Instruction::new(
                "ITERATE_BEGIN",
                &[
                    ("step", step.title.clone()),
                    ("guard", step.guard.clone().unwrap_or_default()),
                    ("has_else", has_else_branch(step).to_string()),
                    (
                        "repeat_while",
                        step.repeat_while.clone().unwrap_or_default(),
                    ),
                    (
                        "repeat_until",
                        step.repeat_until.clone().unwrap_or_default(),
                    ),
                    ("query", query.clone()),
                    (
                        "item",
                        step.iteration_item
                            .clone()
                            .unwrap_or_else(|| "item".to_string()),
                    ),
                ],
            ));
        }
        emit_step_branch(
            &mut instructions,
            step,
            StepBranch {
                call: &step.call,
                invoke: &step.invoke,
                parallel_invokes: &step.parallel_invokes,
                set_statements: &step.set_statements,
                append_statements: &step.append_statements,
                compute_statements: &step.compute_statements,
                delete_statements: &step.delete_statements,
                operation_contract: primary_contract,
                branch: "primary",
            },
        );
        if has_else_branch(step) {
            instructions.push(Instruction::new(
                "ELSE_BRANCH",
                &[("step", step.title.clone())],
            ));
            emit_step_branch(
                &mut instructions,
                step,
                StepBranch {
                    call: &step.otherwise_call,
                    invoke: &step.otherwise_invoke,
                    parallel_invokes: &step.otherwise_parallel_invokes,
                    set_statements: &step.otherwise_set_statements,
                    append_statements: &step.otherwise_append_statements,
                    compute_statements: &step.otherwise_compute_statements,
                    delete_statements: &step.otherwise_delete_statements,
                    operation_contract: else_contract,
                    branch: "else",
                },
            );
        }
        if step.iterate_over.is_some() {
            instructions.push(Instruction::new(
                "ITERATE_END",
                &[("step", step.title.clone())],
            ));
        }
        instructions.push(Instruction::new(
            "END_STEP",
            &[
                ("step", step.title.clone()),
                (
                    "repeat_while",
                    step.repeat_while.clone().unwrap_or_default(),
                ),
                (
                    "repeat_until",
                    step.repeat_until.clone().unwrap_or_default(),
                ),
            ],
        ));
    }

    for handler in &document.intent.failure_handlers {
        instructions.push(Instruction::new(
            "HANDLE_FAILURE",
            &[
                ("condition", handler.condition.clone()),
                (
                    "stop_failure",
                    handler.stop_failure.clone().unwrap_or_default(),
                ),
            ],
        ));
    }

    for guarantee in &document.intent.guarantees {
        for statement in &guarantee.statements {
            instructions.push(Instruction::new(
                "ASSERT_GUARANTEE",
                &[("statement", statement.clone())],
            ));
        }
    }

    instructions.push(Instruction::new("RETURN", &[]));

    BytecodeProgram {
        intent: document.intent.name.clone(),
        document: document.clone(),
        instructions,
        handlers: document
            .intent
            .failure_handlers
            .iter()
            .map(FailureHandler::from_rif)
            .collect(),
    }
}

fn has_else_branch(step: &crate::rif_model::Step) -> bool {
    step.otherwise_call.is_some()
        || step.otherwise_invoke.is_some()
        || !step.otherwise_parallel_invokes.is_empty()
        || !step.otherwise_set_statements.is_empty()
        || !step.otherwise_append_statements.is_empty()
        || !step.otherwise_compute_statements.is_empty()
        || !step.otherwise_delete_statements.is_empty()
}

struct StepBranch<'a> {
    call: &'a Option<crate::rif_model::OperationCall>,
    invoke: &'a Option<InvocationTarget>,
    parallel_invokes: &'a [InvocationTarget],
    set_statements: &'a [String],
    append_statements: &'a [String],
    compute_statements: &'a [String],
    delete_statements: &'a [String],
    operation_contract: Option<&'a crate::rif_model::OperationDefinition>,
    branch: &'a str,
}

fn emit_step_branch(
    instructions: &mut Vec<Instruction>,
    step: &crate::rif_model::Step,
    branch: StepBranch<'_>,
) {
    if let Some(compensation) = &step.compensation
        && branch.branch == "primary"
    {
        instructions.push(Instruction::new(
            "REGISTER_COMPENSATION",
            &[
                ("step", step.title.clone()),
                ("expression", compensation.clone()),
            ],
        ));
    }
    if let Some(call) = branch.call {
        let may_fail = if let Some(operation) = branch.operation_contract {
            let mut failures = std::collections::BTreeSet::new();
            failures.extend(step.may_fail.iter().cloned());
            failures.extend(operation.may_fail.iter().cloned());
            failures.into_iter().collect::<Vec<_>>().join(",")
        } else {
            step.may_fail.join(",")
        };
        instructions.push(Instruction::new(
            "CALL_EFFECT",
            &[
                ("step", step.title.clone()),
                ("target", call.target.clone()),
                ("args", call.args.join(",")),
                ("may_fail", may_fail),
                ("branch", branch.branch.to_string()),
            ],
        ));
    }
    if let Some(target) = branch.invoke {
        instructions.push(Instruction::new(
            "CALL_INTENT",
            &[
                ("step", step.title.clone()),
                ("target", target.target.clone()),
                ("bindings", serialize_bindings(&target.bindings)),
                ("branch", branch.branch.to_string()),
            ],
        ));
    }
    if !branch.parallel_invokes.is_empty() {
        instructions.push(Instruction::new(
            "CALL_INTENT_PARALLEL",
            &[
                ("step", step.title.clone()),
                (
                    "targets",
                    branch
                        .parallel_invokes
                        .iter()
                        .map(serialize_invocation)
                        .collect::<Vec<_>>()
                        .join(","),
                ),
                ("branch", branch.branch.to_string()),
            ],
        ));
    }
    for output in step.outputs.values() {
        instructions.push(Instruction::new(
            "STORE_OUTPUT",
            &[
                ("step", step.title.clone()),
                ("step_number", step.number.to_string()),
                ("name", output.name.clone()),
                ("type", output.type_name.clone()),
                ("branch", branch.branch.to_string()),
            ],
        ));
    }
    for statement in branch.set_statements {
        let (field, value) =
            split_set(statement).unwrap_or_else(|| (statement.clone(), String::new()));
        instructions.push(Instruction::new(
            "SET_FIELD",
            &[
                ("step", step.title.clone()),
                ("field", field),
                ("value", value),
                ("branch", branch.branch.to_string()),
            ],
        ));
    }
    for statement in branch.append_statements {
        let (field, value) =
            split_append(statement).unwrap_or_else(|| (statement.clone(), String::new()));
        instructions.push(Instruction::new(
            "APPEND_LIST",
            &[
                ("step", step.title.clone()),
                ("field", field),
                ("value", value),
                ("branch", branch.branch.to_string()),
            ],
        ));
    }
    for statement in branch.compute_statements {
        let (field, expression) =
            split_set(statement).unwrap_or_else(|| (statement.clone(), String::new()));
        instructions.push(Instruction::new(
            "COMPUTE",
            &[
                ("step", step.title.clone()),
                ("field", field),
                ("expression", expression),
                ("branch", branch.branch.to_string()),
            ],
        ));
    }
    for statement in branch.delete_statements {
        instructions.push(Instruction::new(
            "DELETE_FIELD",
            &[
                ("step", step.title.clone()),
                ("path", statement.clone()),
                ("branch", branch.branch.to_string()),
            ],
        ));
    }
}

pub fn run_bytecode(
    program: &BytecodeProgram,
    initial_state: BTreeMap<String, String>,
    fail_at: BTreeMap<String, String>,
) -> BytecodeRunResult {
    run_bytecode_frame(program, initial_state, BTreeMap::new(), fail_at, 0)
}

fn run_bytecode_frame(
    program: &BytecodeProgram,
    initial_state: BTreeMap<String, String>,
    initial_outputs: BTreeMap<String, String>,
    fail_at: BTreeMap<String, String>,
    invoke_depth: usize,
) -> BytecodeRunResult {
    let mut state = initial_state;
    let mut outputs = initial_outputs;
    let mut trace = Vec::new();
    let mut registered_compensations = Vec::new();
    let mut skipped_steps = std::collections::BTreeSet::new();
    let mut step_starts: BTreeMap<String, usize> = BTreeMap::new();
    let mut repeat_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut branch_modes: BTreeMap<String, BranchMode> = BTreeMap::new();
    let mut branch_sections: BTreeMap<String, BranchSection> = BTreeMap::new();
    let mut pc = 0usize;

    while pc < program.instructions.len() {
        let instruction = &program.instructions[pc];
        match instruction.opcode.as_str() {
            "BEGIN_STEP" => {
                let step = field(instruction, "step");
                let guard = field(instruction, "guard");
                let has_else = field(instruction, "has_else") == "true";
                if guard.is_empty() || evaluate_predicate(guard, &state, &outputs) {
                    trace.push(format!("BEGIN {step}"));
                    step_starts.insert(step.to_string(), pc);
                    branch_modes.insert(step.to_string(), BranchMode::Primary);
                    branch_sections.insert(step.to_string(), BranchSection::Primary);
                } else {
                    if has_else {
                        trace.push(format!("BEGIN {step}"));
                        step_starts.insert(step.to_string(), pc);
                        branch_modes.insert(step.to_string(), BranchMode::Else);
                        branch_sections.insert(step.to_string(), BranchSection::Primary);
                    } else {
                        skipped_steps.insert(step.to_string());
                        branch_modes.insert(step.to_string(), BranchMode::Skipped);
                        branch_sections.insert(step.to_string(), BranchSection::Primary);
                        trace.push(format!("SKIP {step} when {guard}"));
                    }
                }
            }
            "ITERATE_BEGIN" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let step = field(instruction, "step");
                let query = field(instruction, "query");
                let item = field(instruction, "item");
                let guard = field(instruction, "guard");
                let has_else = field(instruction, "has_else") == "true";
                let Some(end_pc) = matching_instruction_index(
                    &program.instructions,
                    pc,
                    "ITERATE_BEGIN",
                    "ITERATE_END",
                ) else {
                    return BytecodeRunResult {
                        status: "failed".to_string(),
                        final_state: state,
                        outputs,
                        trace,
                        failure: Some(format!("UnmatchedIteration:{step}")),
                    };
                };
                let body_program = BytecodeProgram {
                    intent: program.intent.clone(),
                    document: program.document.clone(),
                    instructions: {
                        let mut instructions = Vec::new();
                        instructions.push(Instruction::new(
                            "BEGIN_STEP",
                            &[
                                ("step", step.to_string()),
                                (
                                    "guard",
                                    if branch_modes.get(step).copied() == Some(BranchMode::Else) {
                                        "false".to_string()
                                    } else {
                                        guard.to_string()
                                    },
                                ),
                                ("has_else", has_else.to_string()),
                            ],
                        ));
                        instructions.extend(program.instructions[pc + 1..end_pc].iter().cloned());
                        instructions.push(Instruction::new(
                            "END_STEP",
                            &[
                                ("step", step.to_string()),
                                ("repeat_while", String::new()),
                                ("repeat_until", String::new()),
                            ],
                        ));
                        instructions
                    },
                    handlers: program.handlers.clone(),
                };
                let items = iteration_items(query, &state, &outputs);
                trace.push(format!("ITERATE {query}"));
                for current_item in items {
                    state.insert(item.to_string(), current_item);
                    let result = run_bytecode_frame(
                        &body_program,
                        state.clone(),
                        outputs.clone(),
                        fail_at.clone(),
                        invoke_depth,
                    );
                    trace.extend(result.trace);
                    if result.status == "failed" {
                        return BytecodeRunResult {
                            status: "failed".to_string(),
                            final_state: result.final_state,
                            outputs: result.outputs,
                            trace,
                            failure: result.failure,
                        };
                    }
                    state = result.final_state;
                    outputs = result.outputs;
                }
                pc = end_pc + 1;
                continue;
            }
            "ELSE_BRANCH" => {
                let step = field(instruction, "step");
                if branch_modes
                    .get(step)
                    .is_some_and(|mode| *mode != BranchMode::Skipped)
                {
                    branch_sections.insert(step.to_string(), BranchSection::Else);
                }
            }
            "END_STEP" => {
                let step = field(instruction, "step");
                let repeat_while = field(instruction, "repeat_while");
                let repeat_until = field(instruction, "repeat_until");
                if skipped_steps.remove(step) {
                    repeat_counts.remove(step);
                    step_starts.remove(step);
                    branch_modes.remove(step);
                    branch_sections.remove(step);
                    pc += 1;
                    continue;
                }
                let should_repeat = if !repeat_while.is_empty() {
                    evaluate_predicate(repeat_while, &state, &outputs)
                } else if !repeat_until.is_empty() {
                    !evaluate_predicate(repeat_until, &state, &outputs)
                } else {
                    false
                };
                if should_repeat {
                    let count = repeat_counts.entry(step.to_string()).or_insert(0);
                    *count += 1;
                    if *count > LOOP_LIMIT {
                        trace.push(format!("LOOP LIMIT {step}"));
                        return BytecodeRunResult {
                            status: "failed".to_string(),
                            final_state: state,
                            outputs,
                            trace,
                            failure: Some("LoopLimitExceeded".to_string()),
                        };
                    }
                    if let Some(&start) = step_starts.get(step) {
                        pc = start;
                        continue;
                    }
                }
                repeat_counts.remove(step);
                step_starts.remove(step);
                branch_modes.remove(step);
                branch_sections.remove(step);
            }
            "CHECK_REQUIRES" => {
                let predicate = field(instruction, "predicate");
                if evaluate_predicate(predicate, &state, &outputs) {
                    trace.push(format!("CHECK {predicate}"));
                } else {
                    trace.push(format!("CHECK FAILED {predicate}"));
                    return BytecodeRunResult {
                        status: "failed".to_string(),
                        final_state: state,
                        outputs,
                        trace,
                        failure: Some("RequirementFailed".to_string()),
                    };
                }
            }
            "REGISTER_COMPENSATION" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let expression = field(instruction, "expression").to_string();
                registered_compensations.push(expression.clone());
                trace.push(format!("REGISTER {expression}"));
            }
            "CALL_EFFECT" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let current_step = field(instruction, "step");
                let target = field(instruction, "target");
                trace.push(format!("CALL {target}"));
                if let Some(failure) = forced_failure(current_step, target, &fail_at) {
                    return apply_failure(
                        program,
                        current_step,
                        &failure,
                        state,
                        outputs,
                        trace,
                        registered_compensations,
                    );
                }
            }
            "CALL_INTENT" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let step = field(instruction, "step");
                let target = field(instruction, "target");
                trace.push(format!("CALL INTENT {target}"));
                if invoke_depth >= INVOKE_DEPTH_LIMIT {
                    return BytecodeRunResult {
                        status: "failed".to_string(),
                        final_state: state,
                        outputs,
                        trace,
                        failure: Some("InvokeDepthExceeded".to_string()),
                    };
                }
                let Some(subdocument) = subdocument_for_intent(&program.document, target) else {
                    return BytecodeRunResult {
                        status: "failed".to_string(),
                        final_state: state,
                        outputs,
                        trace,
                        failure: Some(format!("UnknownIntent:{step}")),
                    };
                };
                let invocation = parse_invocation_text(&format!(
                    "{}({})",
                    target,
                    field(instruction, "bindings")
                ));
                let subprogram = lower_document(&subdocument);
                let mut child_state = state.clone();
                apply_invocation_bindings(
                    &invocation,
                    &program.document,
                    &state,
                    &outputs,
                    &mut child_state,
                );
                let mut result = run_bytecode_frame(
                    &subprogram,
                    child_state,
                    BTreeMap::new(),
                    fail_at.clone(),
                    invoke_depth + 1,
                );
                trace.extend(result.trace);
                if result.status == "failed" {
                    return BytecodeRunResult {
                        status: "failed".to_string(),
                        final_state: result.final_state,
                        outputs: result.outputs,
                        trace,
                        failure: result.failure,
                    };
                }
                apply_invocation_result_bindings(
                    &invocation,
                    &program.document,
                    &state,
                    &outputs,
                    &mut result.final_state,
                );
                state = result.final_state;
                outputs.extend(result.outputs);
            }
            "CALL_INTENT_PARALLEL" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let targets = field(instruction, "targets");
                let targets = split_targets(targets)
                    .into_iter()
                    .map(|target| parse_invocation_text(&target))
                    .collect::<Vec<_>>();
                trace.push(format!(
                    "FANOUT {}",
                    targets
                        .iter()
                        .map(serialize_invocation)
                        .collect::<Vec<_>>()
                        .join(",")
                ));
                match run_parallel_invocations(
                    &program.document,
                    &targets,
                    state.clone(),
                    outputs.clone(),
                    fail_at.clone(),
                    invoke_depth,
                ) {
                    Ok((merged_state, merged_outputs, parallel_trace)) => {
                        trace.extend(parallel_trace);
                        state = merged_state;
                        outputs = merged_outputs;
                    }
                    Err(result) => {
                        return BytecodeRunResult {
                            status: "failed".to_string(),
                            final_state: result.final_state,
                            outputs: result.outputs,
                            trace: {
                                let mut full_trace = trace;
                                full_trace.extend(result.trace);
                                full_trace
                            },
                            failure: result.failure,
                        };
                    }
                }
            }
            "STORE_OUTPUT" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let name = field(instruction, "name");
                let step_number = field(instruction, "step_number");
                outputs.insert(name.to_string(), format!("{name}#{step_number}"));
                trace.push(format!("STORE {name}"));
            }
            "SET_FIELD" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let field_name = field(instruction, "field");
                let value = field(instruction, "value");
                if let Some(copied_fields) = apply_typed_object_set(
                    &program.document,
                    field_name,
                    value,
                    &mut state,
                    &outputs,
                ) {
                    for resolved_field_name in copied_fields {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    }
                } else {
                    let evaluated_value =
                        evaluate_typed_expression(&program.document, value, &state, &outputs);
                    if let Some(resolved_field_name) = apply_container_option_value_entry_set(
                        field_name,
                        &evaluated_value,
                        &mut state,
                        &outputs,
                    ) {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) =
                        apply_container_result_variant_entry_set(
                            field_name,
                            &evaluated_value,
                            &mut state,
                            &outputs,
                        )
                    {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) =
                        apply_container_option_object_field_entry_set(
                            field_name,
                            &evaluated_value,
                            &mut state,
                            &outputs,
                        )
                    {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) =
                        apply_container_result_object_field_entry_set(
                            field_name,
                            &evaluated_value,
                            &mut state,
                            &outputs,
                        )
                    {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) = apply_option_value_entry_set(
                        field_name,
                        &evaluated_value,
                        &mut state,
                        &outputs,
                    ) {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) = apply_result_variant_entry_set(
                        field_name,
                        &evaluated_value,
                        &mut state,
                        &outputs,
                    ) {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) = apply_option_object_field_entry_set(
                        field_name,
                        &evaluated_value,
                        &mut state,
                        &outputs,
                    ) {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) = apply_result_object_field_entry_set(
                        field_name,
                        &evaluated_value,
                        &mut state,
                        &outputs,
                    ) {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) =
                        apply_option_container_object_field_entry_set(
                            field_name,
                            &evaluated_value,
                            &mut state,
                            &outputs,
                        )
                    {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) =
                        apply_result_container_object_field_entry_set(
                            field_name,
                            &evaluated_value,
                            &mut state,
                            &outputs,
                        )
                    {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) = apply_option_container_entry_set(
                        field_name,
                        &evaluated_value,
                        &mut state,
                        &outputs,
                    ) {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) = apply_result_container_entry_set(
                        field_name,
                        &evaluated_value,
                        &mut state,
                        &outputs,
                    ) {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) = apply_object_field_entry_set(
                        field_name,
                        &evaluated_value,
                        &mut state,
                        &outputs,
                    ) {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) =
                        apply_map_entry_set(field_name, &evaluated_value, &mut state, &outputs)
                    {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else if let Some(resolved_field_name) =
                        apply_list_entry_set(field_name, &evaluated_value, &mut state, &outputs)
                    {
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    } else {
                        let resolved_field_name = resolve_state_path(field_name, &state, &outputs);
                        state.insert(resolved_field_name.clone(), evaluated_value);
                        trace.push(format!("SET {resolved_field_name} = {value}"));
                    }
                }
            }
            "APPEND_LIST" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let field_name = field(instruction, "field");
                let value = field(instruction, "value");
                let appended_value =
                    evaluate_typed_expression(&program.document, value, &state, &outputs);
                let resolved_field_name = if let Some(resolved_field_name) =
                    apply_container_option_list_append(
                        field_name,
                        &appended_value,
                        &mut state,
                        &outputs,
                    ) {
                    resolved_field_name
                } else if let Some(resolved_field_name) = apply_container_result_list_append(
                    field_name,
                    &appended_value,
                    &mut state,
                    &outputs,
                ) {
                    resolved_field_name
                } else if let Some(resolved_field_name) =
                    apply_option_list_append(field_name, &appended_value, &mut state, &outputs)
                {
                    resolved_field_name
                } else if let Some(resolved_field_name) =
                    apply_result_list_append(field_name, &appended_value, &mut state, &outputs)
                {
                    resolved_field_name
                } else {
                    let resolved_field_name = resolve_state_path(field_name, &state, &outputs);
                    let current = state.get(&resolved_field_name).map(String::as_str);
                    state.insert(
                        resolved_field_name.clone(),
                        append_list_value(current, &appended_value),
                    );
                    resolved_field_name
                };
                trace.push(format!("APPEND {resolved_field_name} += {value}"));
            }
            "COMPUTE" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let field_name = field(instruction, "field");
                let expression = field(instruction, "expression");
                let value =
                    evaluate_typed_expression(&program.document, expression, &state, &outputs);
                if is_local_compute_target(field_name) {
                    outputs.insert(field_name.to_string(), value);
                    trace.push(format!("COMPUTE {field_name} = {expression}"));
                } else {
                    let resolved_field_name = resolve_state_path(field_name, &state, &outputs);
                    state.insert(resolved_field_name.clone(), value);
                    trace.push(format!("COMPUTE {resolved_field_name} = {expression}"));
                }
            }
            "DELETE_FIELD" => {
                if is_skipped(instruction, &skipped_steps, &branch_modes, &branch_sections) {
                    pc += 1;
                    continue;
                }
                let path = field(instruction, "path");
                let resolved_path = if let Some(container_path) =
                    apply_container_option_container_entry_delete(path, &mut state, &outputs)
                {
                    container_path
                } else if let Some(container_path) =
                    apply_container_result_container_entry_delete(path, &mut state, &outputs)
                {
                    container_path
                } else if let Some(container_path) =
                    apply_option_container_entry_delete(path, &mut state, &outputs)
                {
                    container_path
                } else if let Some(container_path) =
                    apply_result_container_entry_delete(path, &mut state, &outputs)
                {
                    container_path
                } else if let Some(container_path) =
                    apply_container_entry_delete(path, &mut state, &outputs)
                {
                    container_path
                } else {
                    let resolved_path = resolve_state_path(path, &state, &outputs);
                    let snapshot = state.clone();
                    if !delete_collection_path_with(&mut state, &resolved_path, |name| {
                        outputs
                            .get(name)
                            .cloned()
                            .or_else(|| snapshot.get(name).cloned())
                    }) {
                        delete_state_path(&mut state, &resolved_path);
                    }
                    resolved_path
                };
                trace.push(format!("DELETE {resolved_path}"));
            }
            "ASSERT_GUARANTEE" => {
                let statement = field(instruction, "statement");
                if evaluate_guarantee(statement, &state, &outputs) {
                    trace.push(format!("ASSERT {statement}"));
                } else {
                    trace.push(format!("ASSERT FAILED {statement}"));
                    return BytecodeRunResult {
                        status: "failed".to_string(),
                        final_state: state,
                        outputs,
                        trace,
                        failure: Some("GuaranteeFailed".to_string()),
                    };
                }
            }
            "RETURN" => {
                publish_returns(&program.document, &state, &mut outputs);
                return BytecodeRunResult {
                    status: "succeeded".to_string(),
                    final_state: state,
                    outputs,
                    trace,
                    failure: None,
                };
            }
            "HANDLE_FAILURE" => {}
            _ => trace.push(format!("UNKNOWN {}", instruction.opcode)),
        }
        pc += 1;
    }

    BytecodeRunResult {
        status: "succeeded".to_string(),
        final_state: {
            publish_returns(&program.document, &state, &mut outputs);
            state
        },
        outputs,
        trace,
        failure: None,
    }
}

fn publish_returns(
    document: &crate::rif_model::RifDocument,
    state: &BTreeMap<String, String>,
    outputs: &mut BTreeMap<String, String>,
) {
    for return_value in &document.intent.returns {
        let output_snapshot = outputs.clone();
        let value = typed_object_return_value(document, &return_value.source, state)
            .unwrap_or_else(|| {
                evaluate_typed_expression(document, &return_value.source, state, &output_snapshot)
            });
        outputs.insert(return_value.name.clone(), value);
    }
}

fn typed_object_return_value(
    document: &crate::rif_model::RifDocument,
    source: &str,
    state: &BTreeMap<String, String>,
) -> Option<String> {
    let source = source.trim();
    let type_name = object_source_type(document, source)?;
    let thing = document.application.things.get(&type_name)?;
    typed_object_state_value(document, source, thing, state)
}

fn typed_object_expression_value(
    document: &crate::rif_model::RifDocument,
    source: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let source = source.trim();
    let type_name = object_source_type(document, source)?;
    let thing = document.application.things.get(&type_name)?;
    typed_object_state_value_with_outputs(document, source, thing, state, outputs)
}

fn object_source_type(document: &crate::rif_model::RifDocument, source: &str) -> Option<String> {
    let (root, field_path) = split_object_path_suffix(source);
    if let Some(collection_type) = collection_record_type(document, root) {
        return if field_path.is_empty() {
            Some(collection_type)
        } else {
            object_field_type(document, &collection_type, field_path)
        };
    }
    let root_type = document
        .intent
        .subjects
        .get(root)
        .or_else(|| document.intent.inputs.get(root))?
        .type_name
        .clone();
    if field_path.is_empty() {
        Some(root_type)
    } else {
        object_field_type(document, &root_type, field_path)
    }
}

fn collection_record_type(document: &crate::rif_model::RifDocument, path: &str) -> Option<String> {
    let (collection_name, _) = path.trim().split_once('[')?;
    document
        .application
        .collections
        .get(collection_name)
        .map(|collection| collection.type_name.clone())
}

fn split_object_path_suffix(path: &str) -> (&str, &str) {
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

fn object_field_type(
    document: &crate::rif_model::RifDocument,
    root_type: &str,
    field_path: &str,
) -> Option<String> {
    let mut current_type = root_type.to_string();
    for field_name in field_path.split('.') {
        let thing = document.application.things.get(&current_type)?;
        let field = thing.fields.get(field_name)?;
        current_type = field.type_name.clone();
    }
    Some(current_type)
}

fn typed_object_state_value(
    document: &crate::rif_model::RifDocument,
    source: &str,
    thing: &crate::rif_model::ThingDefinition,
    state: &BTreeMap<String, String>,
) -> Option<String> {
    let mut entries = Vec::new();
    for field in thing.fields.values() {
        if field.is_secret {
            return None;
        }
        let field_source = format!("{source}.{}", field.name);
        let value = if let Some(nested_thing) = document.application.things.get(&field.type_name) {
            typed_object_state_value(document, &field_source, nested_thing, state)?
        } else {
            state.get(&field_source)?.clone()
        };
        entries.push(format!(
            "{}:{}",
            json_string(&field.name),
            json_value(&value)
        ));
    }
    Some(format!("{{{}}}", entries.join(",")))
}

fn typed_object_state_value_with_outputs(
    document: &crate::rif_model::RifDocument,
    source: &str,
    thing: &crate::rif_model::ThingDefinition,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let mut entries = Vec::new();
    for field in thing.fields.values() {
        if field.is_secret {
            return None;
        }
        let field_source = format!("{source}.{}", field.name);
        let value = if let Some(nested_thing) = document.application.things.get(&field.type_name) {
            typed_object_state_value_with_outputs(
                document,
                &field_source,
                nested_thing,
                state,
                outputs,
            )?
        } else {
            let resolved_source = resolve_state_path(&field_source, state, outputs);
            state
                .get(&resolved_source)
                .or_else(|| outputs.get(&resolved_source))?
                .clone()
        };
        entries.push(format!(
            "{}:{}",
            json_string(&field.name),
            json_value(&value)
        ));
    }
    Some(format!("{{{}}}", entries.join(",")))
}

fn serialize_bindings(bindings: &BTreeMap<String, String>) -> String {
    bindings
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn serialize_invocation(invocation: &InvocationTarget) -> String {
    if invocation.bindings.is_empty() {
        return invocation.target.clone();
    }
    format!(
        "{}({})",
        invocation.target,
        serialize_bindings(&invocation.bindings)
    )
}

fn parse_invocation_text(text: &str) -> InvocationTarget {
    let text = text.trim();
    let Some(open) = text.find('(') else {
        return InvocationTarget {
            target: text.to_string(),
            bindings: BTreeMap::new(),
        };
    };
    let close = text.rfind(')').unwrap_or(text.len());
    let target = text[..open].trim().to_string();
    let bindings_text = &text[open + 1..close];
    let mut bindings = BTreeMap::new();
    for binding in split_targets(bindings_text) {
        if let Some((name, value)) = binding.split_once('=') {
            bindings.insert(name.trim().to_string(), value.trim().to_string());
        }
    }
    InvocationTarget { target, bindings }
}

fn apply_invocation_bindings(
    invocation: &InvocationTarget,
    document: &crate::rif_model::RifDocument,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
    child_state: &mut BTreeMap<String, String>,
) {
    let Some(target_intent) = document
        .intents
        .iter()
        .find(|intent| intent.name == invocation.target)
    else {
        return;
    };
    for (name, expression) in &invocation.bindings {
        if target_intent.subjects.contains_key(name) || target_intent.inputs.contains_key(name) {
            if target_intent.subjects.contains_key(name) {
                let source_prefix = resolve_state_path(expression, state, outputs);
                if copy_state_prefix(state, &source_prefix, child_state, name) {
                    continue;
                }
            }
            let value = evaluate_typed_expression(document, expression, state, outputs);
            child_state.insert(name.clone(), value);
        }
    }
}

fn apply_invocation_result_bindings(
    invocation: &InvocationTarget,
    document: &crate::rif_model::RifDocument,
    parent_state: &BTreeMap<String, String>,
    parent_outputs: &BTreeMap<String, String>,
    child_state: &mut BTreeMap<String, String>,
) {
    let Some(target_intent) = document
        .intents
        .iter()
        .find(|intent| intent.name == invocation.target)
    else {
        return;
    };
    for (target_name, source_expression) in &invocation.bindings {
        if !target_intent.subjects.contains_key(target_name) {
            continue;
        }
        let source_prefix = resolve_state_path(source_expression, parent_state, parent_outputs);
        if target_name == &source_prefix {
            continue;
        }
        copy_state_prefix_from_snapshot(
            child_state.clone(),
            target_name,
            child_state,
            &source_prefix,
        );
        restore_state_prefix(child_state, parent_state, target_name);
    }
}

fn copy_state_prefix(
    source: &BTreeMap<String, String>,
    source_prefix: &str,
    target: &mut BTreeMap<String, String>,
    target_prefix: &str,
) -> bool {
    copy_state_prefix_from_snapshot(source.clone(), source_prefix, target, target_prefix)
}

fn copy_state_prefix_from_snapshot(
    source: BTreeMap<String, String>,
    source_prefix: &str,
    target: &mut BTreeMap<String, String>,
    target_prefix: &str,
) -> bool {
    let mut copied = false;
    let source_dot_prefix = format!("{source_prefix}.");
    let target_dot_prefix = format!("{target_prefix}.");
    for (key, value) in source {
        if key == source_prefix {
            target.insert(target_prefix.to_string(), value);
            copied = true;
        } else if let Some(suffix) = key.strip_prefix(&source_dot_prefix) {
            target.insert(format!("{target_dot_prefix}{suffix}"), value);
            copied = true;
        }
    }
    copied
}

fn restore_state_prefix(
    state: &mut BTreeMap<String, String>,
    original: &BTreeMap<String, String>,
    prefix: &str,
) {
    let dot_prefix = format!("{prefix}.");
    let keys = state
        .keys()
        .filter(|key| *key == prefix || key.starts_with(&dot_prefix))
        .cloned()
        .collect::<Vec<_>>();
    for key in keys {
        state.remove(&key);
    }
    for (key, value) in original {
        if key == prefix || key.starts_with(&dot_prefix) {
            state.insert(key.clone(), value.clone());
        }
    }
}

fn subdocument_for_intent(
    document: &crate::rif_model::RifDocument,
    target: &str,
) -> Option<crate::rif_model::RifDocument> {
    let intent = document
        .intents
        .iter()
        .find(|intent| intent.name == target)?
        .clone();
    Some(crate::rif_model::RifDocument {
        intent: intent.clone(),
        intents: vec![intent],
        application: document.application.clone(),
        source_path: document.source_path.clone(),
    })
}

fn split_targets(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn run_parallel_invocations(
    document: &crate::rif_model::RifDocument,
    targets: &[InvocationTarget],
    base_state: BTreeMap<String, String>,
    base_outputs: BTreeMap<String, String>,
    fail_at: BTreeMap<String, String>,
    invoke_depth: usize,
) -> Result<ParallelJoin, BytecodeRunResult> {
    let mut merged_state = base_state.clone();
    let mut merged_outputs = base_outputs.clone();
    let mut changed_state = BTreeMap::new();
    let mut changed_outputs = BTreeMap::new();
    let mut trace = Vec::new();

    for target in targets {
        trace.push(format!("PARALLEL {}", target.target));
        if invoke_depth >= INVOKE_DEPTH_LIMIT {
            return Err(BytecodeRunResult {
                status: "failed".to_string(),
                final_state: merged_state,
                outputs: merged_outputs,
                trace,
                failure: Some("InvokeDepthExceeded".to_string()),
            });
        }
        let Some(subdocument) = subdocument_for_intent(document, &target.target) else {
            return Err(BytecodeRunResult {
                status: "failed".to_string(),
                final_state: merged_state,
                outputs: merged_outputs,
                trace,
                failure: Some(format!("UnknownIntent:{}", target.target)),
            });
        };
        let subprogram = lower_document(&subdocument);
        let mut child_state = base_state.clone();
        apply_invocation_bindings(
            target,
            document,
            &base_state,
            &base_outputs,
            &mut child_state,
        );
        let mut result = run_bytecode_frame(
            &subprogram,
            child_state,
            BTreeMap::new(),
            fail_at.clone(),
            invoke_depth + 1,
        );
        trace.extend(result.trace);
        if result.status == "failed" {
            return Err(BytecodeRunResult {
                status: "failed".to_string(),
                final_state: result.final_state,
                outputs: result.outputs,
                trace,
                failure: result.failure,
            });
        }
        apply_invocation_result_bindings(
            target,
            document,
            &base_state,
            &base_outputs,
            &mut result.final_state,
        );

        for (key, value) in &result.final_state {
            if base_state.get(key) != Some(value)
                && let Some(existing) = changed_state.insert(key.clone(), value.clone())
                && existing != *value
            {
                return Err(BytecodeRunResult {
                    status: "failed".to_string(),
                    final_state: merged_state,
                    outputs: merged_outputs,
                    trace,
                    failure: Some("ParallelJoinConflict".to_string()),
                });
            }
        }
        for (key, value) in &result.outputs {
            if base_outputs.get(key) != Some(value)
                && let Some(existing) = changed_outputs.insert(key.clone(), value.clone())
                && existing != *value
            {
                return Err(BytecodeRunResult {
                    status: "failed".to_string(),
                    final_state: merged_state,
                    outputs: merged_outputs,
                    trace,
                    failure: Some("ParallelJoinConflict".to_string()),
                });
            }
        }
    }

    for (key, value) in changed_state {
        merged_state.insert(key, value);
    }
    for (key, value) in changed_outputs {
        merged_outputs.insert(key, value);
    }

    Ok((merged_state, merged_outputs, trace))
}

fn matching_instruction_index(
    instructions: &[Instruction],
    start: usize,
    begin_opcode: &str,
    end_opcode: &str,
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, instruction) in instructions.iter().enumerate().skip(start + 1) {
        if instruction.opcode == begin_opcode {
            depth += 1;
        } else if instruction.opcode == end_opcode {
            if depth == 0 {
                return Some(index);
            }
            depth -= 1;
        }
    }
    None
}

fn iteration_items(
    query: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Vec<String> {
    let collection_items = collection_query_keys_with(state, query, |name| {
        outputs
            .get(name)
            .cloned()
            .or_else(|| state.get(name).cloned())
    });
    if !collection_items.is_empty() {
        return collection_items;
    }

    let value = evaluate_expression(query, state, outputs);
    expression::list_literal_values(&value)
        .or_else(|| expression::map_literal_keys(&value))
        .unwrap_or_default()
}

fn is_skipped(
    instruction: &Instruction,
    skipped_steps: &std::collections::BTreeSet<String>,
    branch_modes: &BTreeMap<String, BranchMode>,
    branch_sections: &BTreeMap<String, BranchSection>,
) -> bool {
    let Some(step) = instruction.fields.get("step") else {
        return false;
    };
    if skipped_steps.contains(step) {
        return true;
    }
    let Some(mode) = branch_modes.get(step).copied() else {
        return false;
    };
    let section = instruction
        .fields
        .get("branch")
        .map(|branch| match branch.as_str() {
            "else" => BranchSection::Else,
            _ => BranchSection::Primary,
        })
        .or_else(|| branch_sections.get(step).copied())
        .unwrap_or(BranchSection::Primary);
    match mode {
        BranchMode::Primary => section == BranchSection::Else,
        BranchMode::Else => section == BranchSection::Primary,
        BranchMode::Skipped => true,
    }
}

fn apply_failure(
    program: &BytecodeProgram,
    step: &str,
    failure: &str,
    mut state: BTreeMap<String, String>,
    outputs: BTreeMap<String, String>,
    mut trace: Vec<String>,
    registered_compensations: Vec<String>,
) -> BytecodeRunResult {
    let mut stop_failure = failure.to_string();
    let handler = find_handler(&program.handlers, step, failure);
    if let Some(handler) = handler {
        if handler.actions.iter().any(|action| {
            action
                .strip_prefix("ignore ")
                .is_some_and(|ignored| ignored.trim() == failure)
        }) {
            trace.push(format!("IGNORE {failure}"));
            return BytecodeRunResult {
                status: "succeeded".to_string(),
                final_state: state,
                outputs,
                trace,
                failure: None,
            };
        }

        for action in &handler.actions {
            if let Some(statement) = action.strip_prefix("set ") {
                apply_set(&program.document, statement.trim(), &mut state, &outputs);
                trace.push(action.clone());
            } else if let Some(stop) = action.strip_prefix("stop with ") {
                stop_failure = stop.trim().to_string();
                trace.push(action.clone());
            } else {
                trace.push(action.clone());
            }
        }
    } else {
        for compensation in registered_compensations.iter().rev() {
            trace.push(format!("compensate {compensation}"));
        }
    }

    BytecodeRunResult {
        status: "failed".to_string(),
        final_state: state,
        outputs,
        trace,
        failure: Some(stop_failure),
    }
}

fn find_handler<'a>(
    handlers: &'a [FailureHandler],
    step: &str,
    failure: &str,
) -> Option<&'a FailureHandler> {
    let failure_tokens = tokens(
        &failure
            .replace("Failed", "")
            .replace("Failure", "")
            .replace("Invalid", ""),
    );
    let step_tokens = tokens(step);
    handlers.iter().find(|handler| {
        handler.stop_failure.as_deref() == Some(failure)
            || handler
                .ignored_failures
                .iter()
                .any(|ignored| ignored == failure)
            || !failure_tokens.is_disjoint(&tokens(&handler.condition))
            || (handler.condition.to_ascii_lowercase().contains("fail")
                && !step_tokens.is_disjoint(&tokens(&handler.condition)))
    })
}

fn forced_failure(step: &str, target: &str, fail_at: &BTreeMap<String, String>) -> Option<String> {
    fail_at
        .get(step)
        .cloned()
        .or_else(|| fail_at.get(target).cloned())
}

fn evaluate_predicate(
    predicate: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    predicate::evaluate(predicate, state, outputs)
}

fn evaluate_guarantee(
    statement: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> bool {
    if let Some(reference) = statement.strip_suffix(" exists") {
        return lookup_value(reference.trim(), state, outputs).is_some();
    }
    predicate::evaluate(statement, state, outputs)
}

fn lookup_value(
    name: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    if let Some(value) =
        expression::resolve_container_count(name, |key| lookup_value(key, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_map_lookup(name, |key| lookup_value(key, state, outputs))
    {
        return Some(value);
    }
    let name = resolve_state_path(name, state, outputs);
    state.get(&name).or_else(|| outputs.get(&name)).cloned()
}

fn evaluate_expression(
    expression: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> String {
    expression::evaluate(expression, |token| expression_value(token, state, outputs))
}

fn evaluate_typed_expression(
    document: &crate::rif_model::RifDocument,
    expression: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> String {
    expression::evaluate(expression, |token| {
        typed_object_expression_value(document, token, state, outputs)
            .or_else(|| expression_value(token, state, outputs))
    })
}

fn expression_value(
    token: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    if let Some(value) = collection_path_value_with(state, token, |name| {
        outputs
            .get(name)
            .cloned()
            .or_else(|| state.get(name).cloned())
    }) {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_container_count(token, |name| expression_value(name, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) = expression::resolve_option_value_lookup(token, |name| {
        expression_value(name, state, outputs)
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_result_variant_lookup(token, |name| {
        expression_value(name, state, outputs)
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_object_field_lookup(token, |name| {
        expression_value(name, state, outputs)
    }) {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_map_lookup(token, |name| expression_value(name, state, outputs))
    {
        return Some(value);
    }
    if let Some(value) =
        expression::resolve_list_lookup(token, |name| expression_value(name, state, outputs))
    {
        return Some(value);
    }
    let token = resolve_state_path(token, state, outputs);
    state.get(&token).or_else(|| outputs.get(&token)).cloned()
}

fn apply_set(
    document: &crate::rif_model::RifDocument,
    statement: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) {
    if let Some((left, right)) = split_set(statement) {
        if apply_typed_object_set(document, &left, &right, state, outputs).is_some() {
            return;
        }
        let value = evaluate_typed_expression(document, &right, state, outputs);
        if apply_option_value_entry_set(&left, &value, state, outputs).is_none()
            && apply_result_variant_entry_set(&left, &value, state, outputs).is_none()
            && apply_option_object_field_entry_set(&left, &value, state, outputs).is_none()
            && apply_result_object_field_entry_set(&left, &value, state, outputs).is_none()
            && apply_option_container_object_field_entry_set(&left, &value, state, outputs)
                .is_none()
            && apply_result_container_object_field_entry_set(&left, &value, state, outputs)
                .is_none()
            && apply_option_container_entry_set(&left, &value, state, outputs).is_none()
            && apply_result_container_entry_set(&left, &value, state, outputs).is_none()
            && apply_object_field_entry_set(&left, &value, state, outputs).is_none()
            && apply_map_entry_set(&left, &value, state, outputs).is_none()
            && apply_list_entry_set(&left, &value, state, outputs).is_none()
        {
            let left = resolve_state_path(&left, state, outputs);
            state.insert(left, value);
        }
    }
}

fn apply_typed_object_set(
    document: &crate::rif_model::RifDocument,
    target: &str,
    source: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<Vec<String>> {
    let target = target.trim();
    let source = source.trim();
    let target_type = object_source_type(document, target)?;
    let source_type = object_source_type(document, source)?;
    if target_type != source_type {
        return None;
    }
    let thing = document.application.things.get(&target_type)?;
    copy_typed_object_fields(document, target, source, thing, state, outputs)
}

fn copy_typed_object_fields(
    document: &crate::rif_model::RifDocument,
    target: &str,
    source: &str,
    thing: &crate::rif_model::ThingDefinition,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<Vec<String>> {
    let mut changed_fields = Vec::new();
    for field in thing.fields.values() {
        let target_field = format!("{target}.{}", field.name);
        let source_field = format!("{source}.{}", field.name);
        if let Some(nested_thing) = document.application.things.get(&field.type_name) {
            changed_fields.extend(copy_typed_object_fields(
                document,
                &target_field,
                &source_field,
                nested_thing,
                state,
                outputs,
            )?);
        } else {
            let resolved_source = resolve_state_path(&source_field, state, outputs);
            let value = state
                .get(&resolved_source)
                .or_else(|| outputs.get(&resolved_source))?
                .clone();
            let resolved_target = resolve_state_path(&target_field, state, outputs);
            state.insert(resolved_target.clone(), value);
            changed_fields.push(resolved_target);
        }
    }
    Some(changed_fields)
}

fn apply_map_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (map_expression, key_expression) = expression::split_map_lookup(target)?;
    let map_key = resolve_state_path(map_expression, state, outputs);
    let current_map = state
        .get(&map_key)
        .cloned()
        .or_else(|| outputs.get(&map_key).cloned())?;
    let lookup_key = evaluate_expression(key_expression, state, outputs);
    let updated_map = expression::set_map_lookup_value(&current_map, &lookup_key, value)?;
    state.insert(map_key.clone(), updated_map);
    Some(map_key)
}

fn apply_container_option_value_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let entry_expression = target.trim().strip_suffix(".value")?.trim();
    let (container_expression, index_expression) =
        expression::split_index_lookup(entry_expression)?;
    let container_key = resolve_state_path(container_expression, state, outputs);
    let current_container = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())?;
    let index = evaluate_expression(index_expression, state, outputs);
    let current_entry =
        container_entry(&current_container, &index).unwrap_or_else(|| "None".to_string());
    let updated_entry = expression::set_option_value(&current_entry, &json_value(value))?;
    let updated_container = set_container_entry(&current_container, &index, &updated_entry)?;
    state.insert(container_key.clone(), updated_container);
    Some(container_key)
}

fn apply_container_result_variant_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (entry_expression, variant) = expression::split_result_variant_projection(target)?;
    let (container_expression, index_expression) =
        expression::split_index_lookup(entry_expression)?;
    let container_key = resolve_state_path(container_expression, state, outputs);
    let current_container = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())?;
    let index = evaluate_expression(index_expression, state, outputs);
    let current_entry = container_entry(&current_container, &index)?;
    let updated_entry =
        expression::set_result_variant_value(&current_entry, variant, &json_value(value))?;
    let updated_container = set_container_entry(&current_container, &index, &updated_entry)?;
    state.insert(container_key.clone(), updated_container);
    Some(container_key)
}

fn apply_container_option_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (entry_expression, field_path) = expression::split_option_value_field_projection(target)?;
    let (container_expression, index_expression) =
        expression::split_index_lookup(entry_expression)?;
    let container_key = resolve_state_path(container_expression, state, outputs);
    let current_container = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())?;
    let index = evaluate_expression(index_expression, state, outputs);
    let current_entry = container_entry(&current_container, &index)?;
    let value_projection = format!("{entry_expression}.value");
    let current_object = expression::resolve_option_value_lookup(&value_projection, |name| {
        (name == entry_expression).then(|| current_entry.clone())
    })?;
    let updated_object =
        expression::set_object_field_value(&current_object, field_path, &json_value(value))?;
    let updated_entry = expression::set_option_value(&current_entry, &updated_object)?;
    let updated_container = set_container_entry(&current_container, &index, &updated_entry)?;
    state.insert(container_key.clone(), updated_container);
    Some(container_key)
}

fn apply_container_result_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (entry_expression, variant, field_path) =
        expression::split_result_variant_field_projection(target)?;
    let (container_expression, index_expression) =
        expression::split_index_lookup(entry_expression)?;
    let container_key = resolve_state_path(container_expression, state, outputs);
    let current_container = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())?;
    let index = evaluate_expression(index_expression, state, outputs);
    let current_entry = container_entry(&current_container, &index)?;
    let variant_projection = format!("{entry_expression}.{variant}");
    let current_object = expression::resolve_result_variant_lookup(&variant_projection, |name| {
        (name == entry_expression).then(|| current_entry.clone())
    })?;
    let updated_object =
        expression::set_object_field_value(&current_object, field_path, &json_value(value))?;
    let updated_entry =
        expression::set_result_variant_value(&current_entry, variant, &updated_object)?;
    let updated_container = set_container_entry(&current_container, &index, &updated_entry)?;
    state.insert(container_key.clone(), updated_container);
    Some(container_key)
}

fn apply_option_value_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let option_expression = target.trim().strip_suffix(".value")?.trim();
    let option_key = resolve_state_path(option_expression, state, outputs);
    let current_option = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())?;
    let updated_option = expression::set_option_value(&current_option, &json_value(value))?;
    state.insert(option_key.clone(), updated_option);
    Some(option_key)
}

fn apply_result_variant_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (result_expression, variant) = expression::split_result_variant_projection(target)?;
    let result_key = resolve_state_path(result_expression, state, outputs);
    let current_result = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())?;
    let updated_result =
        expression::set_result_variant_value(&current_result, variant, &json_value(value))?;
    state.insert(result_key.clone(), updated_result);
    Some(result_key)
}

fn apply_option_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (option_expression, field_path) = expression::split_option_value_field_projection(target)?;
    let option_key = resolve_state_path(option_expression, state, outputs);
    let current_option = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())?;
    let value_projection = format!("{option_expression}.value");
    let current_object = expression::resolve_option_value_lookup(&value_projection, |name| {
        let key = resolve_state_path(name, state, outputs);
        state
            .get(&key)
            .cloned()
            .or_else(|| outputs.get(&key).cloned())
    })?;
    let updated_object =
        expression::set_object_field_value(&current_object, field_path, &json_value(value))?;
    let updated_option = expression::set_option_value(&current_option, &updated_object)?;
    state.insert(option_key.clone(), updated_option);
    Some(option_key)
}

fn apply_result_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (result_expression, variant, field_path) =
        expression::split_result_variant_field_projection(target)?;
    let result_key = resolve_state_path(result_expression, state, outputs);
    let current_result = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())?;
    let variant_projection = format!("{result_expression}.{variant}");
    let current_object = expression::resolve_result_variant_lookup(&variant_projection, |name| {
        let key = resolve_state_path(name, state, outputs);
        state
            .get(&key)
            .cloned()
            .or_else(|| outputs.get(&key).cloned())
    })?;
    let updated_object =
        expression::set_object_field_value(&current_object, field_path, &json_value(value))?;
    let updated_result =
        expression::set_result_variant_value(&current_result, variant, &updated_object)?;
    state.insert(result_key.clone(), updated_result);
    Some(result_key)
}

fn apply_option_container_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (object_expression, field_path) = expression::split_last_top_level_dot(target)?;
    let (container_projection, index_expression) =
        expression::split_index_lookup(object_expression)?;
    let option_expression = container_projection.trim().strip_suffix(".value")?.trim();
    let option_key = resolve_state_path(option_expression, state, outputs);
    let current_option = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())?;
    let value_projection = format!("{option_expression}.value");
    let current_container = expression::resolve_option_value_lookup(&value_projection, |name| {
        let key = resolve_state_path(name, state, outputs);
        state
            .get(&key)
            .cloned()
            .or_else(|| outputs.get(&key).cloned())
    })?;
    let index = evaluate_expression(index_expression, state, outputs);
    let rendered_value = json_value(value);
    let updated_container =
        set_container_object_field(&current_container, &index, field_path, &rendered_value)?;
    let updated_option = expression::set_option_value(&current_option, &updated_container)?;
    state.insert(option_key.clone(), updated_option);
    Some(option_key)
}

fn apply_result_container_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (object_expression, field_path) = expression::split_last_top_level_dot(target)?;
    let (container_projection, index_expression) =
        expression::split_index_lookup(object_expression)?;
    let (result_expression, variant) =
        expression::split_result_variant_projection(container_projection)?;
    let result_key = resolve_state_path(result_expression, state, outputs);
    let current_result = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())?;
    let current_container =
        expression::resolve_result_variant_lookup(container_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })?;
    let index = evaluate_expression(index_expression, state, outputs);
    let rendered_value = json_value(value);
    let updated_container =
        set_container_object_field(&current_container, &index, field_path, &rendered_value)?;
    let updated_result =
        expression::set_result_variant_value(&current_result, variant, &updated_container)?;
    state.insert(result_key.clone(), updated_result);
    Some(result_key)
}

fn apply_option_container_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (container_projection, index_expression) = expression::split_index_lookup(target)?;
    let option_expression = container_projection.trim().strip_suffix(".value")?.trim();
    let option_key = resolve_state_path(option_expression, state, outputs);
    let current_option = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())?;
    let value_projection = format!("{option_expression}.value");
    let current_container = expression::resolve_option_value_lookup(&value_projection, |name| {
        let key = resolve_state_path(name, state, outputs);
        state
            .get(&key)
            .cloned()
            .or_else(|| outputs.get(&key).cloned())
    })?;
    let index = evaluate_expression(index_expression, state, outputs);
    let updated_container = set_container_entry(&current_container, &index, value)?;
    let updated_option = expression::set_option_value(&current_option, &updated_container)?;
    state.insert(option_key.clone(), updated_option);
    Some(option_key)
}

fn apply_result_container_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (container_projection, index_expression) = expression::split_index_lookup(target)?;
    let (result_expression, variant) =
        expression::split_result_variant_projection(container_projection)?;
    let result_key = resolve_state_path(result_expression, state, outputs);
    let current_result = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())?;
    let current_container =
        expression::resolve_result_variant_lookup(container_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })?;
    let index = evaluate_expression(index_expression, state, outputs);
    let updated_container = set_container_entry(&current_container, &index, value)?;
    let updated_result =
        expression::set_result_variant_value(&current_result, variant, &updated_container)?;
    state.insert(result_key.clone(), updated_result);
    Some(result_key)
}

fn apply_container_option_list_append(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let entry_expression = target.trim().strip_suffix(".value")?.trim();
    let (container_expression, index_expression) =
        expression::split_index_lookup(entry_expression)?;
    let container_key = resolve_state_path(container_expression, state, outputs);
    let current_container = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())?;
    let index = evaluate_expression(index_expression, state, outputs);
    let current_entry =
        container_entry(&current_container, &index).unwrap_or_else(|| "None".to_string());
    let current_list = if current_entry.trim() == "None" {
        None
    } else {
        let value_projection = format!("{entry_expression}.value");
        Some(expression::resolve_option_value_lookup(
            &value_projection,
            |name| (name == entry_expression).then(|| current_entry.clone()),
        )?)
    };
    let updated_list = append_list_value(current_list.as_deref(), value);
    let updated_entry = expression::set_option_value(&current_entry, &updated_list)?;
    let updated_container = set_container_entry(&current_container, &index, &updated_entry)?;
    state.insert(container_key.clone(), updated_container);
    Some(container_key)
}

fn apply_container_result_list_append(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (entry_expression, variant) = expression::split_result_variant_projection(target)?;
    let (container_expression, index_expression) =
        expression::split_index_lookup(entry_expression)?;
    let container_key = resolve_state_path(container_expression, state, outputs);
    let current_container = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())?;
    let index = evaluate_expression(index_expression, state, outputs);
    let current_entry = container_entry(&current_container, &index)?;
    let variant_projection = format!("{entry_expression}.{variant}");
    let current_list = expression::resolve_result_variant_lookup(&variant_projection, |name| {
        (name == entry_expression).then(|| current_entry.clone())
    });
    let updated_list = append_list_value(current_list.as_deref(), value);
    let updated_entry =
        expression::set_result_variant_value(&current_entry, variant, &updated_list)?;
    let updated_container = set_container_entry(&current_container, &index, &updated_entry)?;
    state.insert(container_key.clone(), updated_container);
    Some(container_key)
}

fn apply_option_list_append(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let option_expression = target.trim().strip_suffix(".value")?.trim();
    let option_key = resolve_state_path(option_expression, state, outputs);
    let current_option = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())?;
    let current_list = if current_option.trim() == "None" {
        None
    } else {
        Some(expression::resolve_option_value_lookup(target, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })?)
    };
    let updated_list = append_list_value(current_list.as_deref(), value);
    let updated_option = expression::set_option_value(&current_option, &updated_list)?;
    state.insert(option_key.clone(), updated_option);
    Some(option_key)
}

fn apply_result_list_append(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (result_expression, variant) = expression::split_result_variant_projection(target)?;
    let result_key = resolve_state_path(result_expression, state, outputs);
    let current_result = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())?;
    let current_list = expression::resolve_result_variant_lookup(target, |name| {
        let key = resolve_state_path(name, state, outputs);
        state
            .get(&key)
            .cloned()
            .or_else(|| outputs.get(&key).cloned())
    });
    let updated_list = append_list_value(current_list.as_deref(), value);
    let updated_result =
        expression::set_result_variant_value(&current_result, variant, &updated_list)?;
    state.insert(result_key.clone(), updated_result);
    Some(result_key)
}

fn apply_object_field_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (object_expression, field_name) = expression::split_last_top_level_dot(target)?;
    let (container_expression, index_expression) =
        expression::split_index_lookup(object_expression)?;
    let container_key = resolve_state_path(container_expression, state, outputs);
    let current_container = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())?;
    let index = evaluate_expression(index_expression, state, outputs);
    let rendered_value = json_value(value);
    let updated_container =
        set_container_object_field(&current_container, &index, field_name, &rendered_value)?;
    state.insert(container_key.clone(), updated_container);
    Some(container_key)
}

fn set_container_object_field(
    container: &str,
    index: &str,
    field_path: &str,
    rendered_value: &str,
) -> Option<String> {
    if let Some(current_object) = expression::list_lookup_value(container, index) {
        let updated_object =
            expression::set_object_field_value(&current_object, field_path, rendered_value)?;
        return expression::set_list_lookup_value(container, index, &updated_object);
    }
    let current_object = expression::map_lookup_value(container, index)?;
    let updated_object =
        expression::set_object_field_value(&current_object, field_path, rendered_value)?;
    expression::set_map_lookup_value(container, index, &updated_object)
}

fn set_container_entry(container: &str, index: &str, value: &str) -> Option<String> {
    if let Some(updated_map) = expression::set_map_lookup_value(container, index, value) {
        return Some(updated_map);
    }
    expression::set_list_lookup_value(container, index, value)
}

fn container_entry(container: &str, index: &str) -> Option<String> {
    if let Some(value) = expression::map_lookup_value(container, index) {
        return Some(value);
    }
    expression::list_lookup_value(container, index)
}

fn remove_container_entry(container: &str, index: &str) -> Option<String> {
    if let Some(updated_map) = expression::remove_map_lookup_value(container, index) {
        return Some(updated_map);
    }
    expression::remove_list_lookup_value(container, index)
}

fn apply_list_entry_set(
    target: &str,
    value: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (list_expression, index_expression) = expression::split_index_lookup(target)?;
    let list_key = resolve_state_path(list_expression, state, outputs);
    let current_list = state
        .get(&list_key)
        .cloned()
        .or_else(|| outputs.get(&list_key).cloned())?;
    let index = evaluate_expression(index_expression, state, outputs);
    let updated_list = expression::set_list_lookup_value(&current_list, &index, value)?;
    state.insert(list_key.clone(), updated_list);
    Some(list_key)
}

fn apply_container_option_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (container_projection, index_expression) = expression::split_index_lookup(target)?;
    let entry_expression = container_projection.trim().strip_suffix(".value")?.trim();
    let (outer_container_expression, outer_index_expression) =
        expression::split_index_lookup(entry_expression)?;
    let outer_container_key = resolve_state_path(outer_container_expression, state, outputs);
    let current_outer_container = state
        .get(&outer_container_key)
        .cloned()
        .or_else(|| outputs.get(&outer_container_key).cloned())?;
    let outer_index = evaluate_expression(outer_index_expression, state, outputs);
    let current_entry = container_entry(&current_outer_container, &outer_index)
        .unwrap_or_else(|| "None".to_string());
    if current_entry.trim() == "None" {
        return Some(outer_container_key);
    }
    let current_inner_container =
        expression::resolve_option_value_lookup(container_projection, |name| {
            (name == entry_expression).then(|| current_entry.clone())
        })?;
    let index = evaluate_expression(index_expression, state, outputs);
    let updated_inner_container = remove_container_entry(&current_inner_container, &index)?;
    let updated_entry = expression::set_option_value(&current_entry, &updated_inner_container)?;
    let updated_outer_container =
        set_container_entry(&current_outer_container, &outer_index, &updated_entry)?;
    state.insert(outer_container_key.clone(), updated_outer_container);
    Some(outer_container_key)
}

fn apply_container_result_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (container_projection, index_expression) = expression::split_index_lookup(target)?;
    let (entry_expression, variant) =
        expression::split_result_variant_projection(container_projection)?;
    let (outer_container_expression, outer_index_expression) =
        expression::split_index_lookup(entry_expression)?;
    let outer_container_key = resolve_state_path(outer_container_expression, state, outputs);
    let current_outer_container = state
        .get(&outer_container_key)
        .cloned()
        .or_else(|| outputs.get(&outer_container_key).cloned())?;
    let outer_index = evaluate_expression(outer_index_expression, state, outputs);
    let current_entry = container_entry(&current_outer_container, &outer_index)?;
    let Some(current_inner_container) =
        expression::resolve_result_variant_lookup(container_projection, |name| {
            (name == entry_expression).then(|| current_entry.clone())
        })
    else {
        return Some(outer_container_key);
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let updated_inner_container = remove_container_entry(&current_inner_container, &index)?;
    let updated_entry =
        expression::set_result_variant_value(&current_entry, variant, &updated_inner_container)?;
    let updated_outer_container =
        set_container_entry(&current_outer_container, &outer_index, &updated_entry)?;
    state.insert(outer_container_key.clone(), updated_outer_container);
    Some(outer_container_key)
}

fn apply_option_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (container_projection, index_expression) = expression::split_index_lookup(target)?;
    let option_expression = container_projection.trim().strip_suffix(".value")?.trim();
    let option_key = resolve_state_path(option_expression, state, outputs);
    let current_option = state
        .get(&option_key)
        .cloned()
        .or_else(|| outputs.get(&option_key).cloned())?;
    if current_option.trim() == "None" {
        return Some(option_key);
    }
    let current_container =
        expression::resolve_option_value_lookup(container_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })?;
    let index = evaluate_expression(index_expression, state, outputs);
    let updated_container = remove_container_entry(&current_container, &index)?;
    let updated_option = expression::set_option_value(&current_option, &updated_container)?;
    state.insert(option_key.clone(), updated_option);
    Some(option_key)
}

fn apply_result_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (container_projection, index_expression) = expression::split_index_lookup(target)?;
    let (result_expression, variant) =
        expression::split_result_variant_projection(container_projection)?;
    let result_key = resolve_state_path(result_expression, state, outputs);
    let current_result = state
        .get(&result_key)
        .cloned()
        .or_else(|| outputs.get(&result_key).cloned())?;
    let Some(current_container) =
        expression::resolve_result_variant_lookup(container_projection, |name| {
            let key = resolve_state_path(name, state, outputs);
            state
                .get(&key)
                .cloned()
                .or_else(|| outputs.get(&key).cloned())
        })
    else {
        return Some(result_key);
    };
    let index = evaluate_expression(index_expression, state, outputs);
    let updated_container = remove_container_entry(&current_container, &index)?;
    let updated_result =
        expression::set_result_variant_value(&current_result, variant, &updated_container)?;
    state.insert(result_key.clone(), updated_result);
    Some(result_key)
}

fn apply_container_entry_delete(
    target: &str,
    state: &mut BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> Option<String> {
    let (container_expression, index_expression) = expression::split_index_lookup(target)?;
    let container_key = resolve_state_path(container_expression, state, outputs);
    let current_value = state
        .get(&container_key)
        .cloned()
        .or_else(|| outputs.get(&container_key).cloned())?;
    let index = evaluate_expression(index_expression, state, outputs);
    let updated_value = remove_container_entry(&current_value, &index)?;
    state.insert(container_key.clone(), updated_value);
    Some(container_key)
}

fn resolve_state_path(
    token: &str,
    state: &BTreeMap<String, String>,
    outputs: &BTreeMap<String, String>,
) -> String {
    if !token.contains('[') {
        return token.to_string();
    }

    let mut resolved = String::new();
    let mut rest = token;
    while let Some(open) = rest.find('[') {
        resolved.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            return token.to_string();
        };
        let inner = &after_open[..close];
        if inner.contains('=') {
            resolved.push('[');
            resolved.push_str(inner);
            resolved.push(']');
            rest = &after_open[close + 1..];
            continue;
        }
        let value = evaluate_expression(inner.trim(), state, outputs);
        if !resolved.is_empty() && !resolved.ends_with('.') {
            resolved.push('.');
        }
        resolved.push_str(&value);
        rest = &after_open[close + 1..];
    }
    resolved.push_str(rest);
    resolved
}

fn delete_state_path(state: &mut BTreeMap<String, String>, path: &str) {
    let prefix = format!("{path}.");
    state.retain(|key, _| key != path && !key.starts_with(&prefix));
}

fn append_list_value(current: Option<&str>, value: &str) -> String {
    let Some(current) = current.map(str::trim).filter(|value| !value.is_empty()) else {
        return format!("[{value}]");
    };
    let Some(inner) = current
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
    else {
        return format!("[{current},{value}]");
    };
    if inner.trim().is_empty() {
        format!("[{value}]")
    } else {
        format!("[{inner},{value}]")
    }
}

fn is_local_compute_target(value: &str) -> bool {
    let mut chars = value.trim().chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn split_set(statement: &str) -> Option<(String, String)> {
    let (left, right) = statement.split_once('=')?;
    Some((left.trim().to_string(), right.trim().to_string()))
}

fn split_append(statement: &str) -> Option<(String, String)> {
    let (left, right) = statement.split_once("+=")?;
    Some((left.trim().to_string(), right.trim().to_string()))
}

fn field<'a>(instruction: &'a Instruction, key: &str) -> &'a str {
    instruction.fields.get(key).map_or("", String::as_str)
}

fn tokens(text: &str) -> BTreeSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphabetic())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| !matches!(token.as_str(), "with" | "failed" | "fails" | "failure"))
        .collect()
}

fn string_map_json(map: &BTreeMap<String, String>) -> String {
    format!(
        "{{{}}}",
        map.iter()
            .map(|(key, value)| format!("{}:{}", json_string(key), json_string(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
