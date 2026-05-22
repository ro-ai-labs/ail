use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thing {
    pub name: String,
    pub type_name: String,
    pub is_secret: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Application {
    pub name: Option<String>,
    pub module: Option<String>,
    pub exports: Vec<ExportDefinition>,
    pub enums: BTreeMap<String, EnumDefinition>,
    pub things: BTreeMap<String, ThingDefinition>,
    pub operations: BTreeMap<String, OperationDefinition>,
    pub collections: BTreeMap<String, CollectionDefinition>,
    pub endpoints: Vec<EndpointDefinition>,
    pub triggers: Vec<TriggerDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportDefinition {
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDefinition {
    pub name: String,
    pub values: Vec<String>,
}

impl EnumDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThingDefinition {
    pub name: String,
    pub fields: BTreeMap<String, FieldDefinition>,
}

impl ThingDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionDefinition {
    pub name: String,
    pub type_name: String,
    pub unique_fields: Vec<String>,
}

impl CollectionDefinition {
    pub fn new(name: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_name: type_name.into(),
            unique_fields: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDefinition {
    pub name: String,
    pub type_name: String,
    pub is_secret: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationDefinition {
    pub name: String,
    pub inputs: BTreeMap<String, String>,
    pub input_order: Vec<String>,
    pub outputs: Vec<OutputValue>,
    pub reads: Vec<String>,
    pub changes: Vec<String>,
    pub external_calls: Vec<String>,
    pub may_fail: Vec<String>,
}

impl OperationDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            inputs: BTreeMap::new(),
            input_order: Vec::new(),
            outputs: Vec::new(),
            reads: Vec::new(),
            changes: Vec::new(),
            external_calls: Vec::new(),
            may_fail: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub owner: String,
    pub name: String,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSet {
    pub field_path: String,
    pub states: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransition {
    pub field_path: String,
    pub from_state: String,
    pub to_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCall {
    pub expression: String,
    pub target: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationTarget {
    pub target: String,
    pub bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointDefinition {
    pub method: String,
    pub path: String,
    pub target: String,
    pub request_fields: BTreeMap<String, String>,
    pub requires: Vec<String>,
    pub bindings: BTreeMap<String, String>,
    pub response_status: Option<String>,
    pub response_fields: BTreeMap<String, String>,
    pub responses: BTreeMap<String, String>,
    pub error_status: Option<String>,
    pub error_fields: BTreeMap<String, String>,
    pub error_responses: BTreeMap<String, String>,
    pub error_cases: BTreeMap<String, EndpointErrorDefinition>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EndpointErrorDefinition {
    pub status: Option<String>,
    pub response_fields: BTreeMap<String, String>,
    pub responses: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerDefinition {
    pub name: String,
    pub target: String,
    pub schedule: Option<String>,
    pub queue: Option<String>,
    pub payload_fields: BTreeMap<String, String>,
    pub requires: Vec<String>,
    pub bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputValue {
    pub name: String,
    pub type_name: String,
    pub is_secret: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnValue {
    pub name: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub name: String,
    pub call: Option<OperationCall>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub number: usize,
    pub title: String,
    pub guard: Option<String>,
    pub repeat_while: Option<String>,
    pub repeat_until: Option<String>,
    pub call: Option<OperationCall>,
    pub otherwise_call: Option<OperationCall>,
    pub invoke: Option<InvocationTarget>,
    pub otherwise_invoke: Option<InvocationTarget>,
    pub parallel_invokes: Vec<InvocationTarget>,
    pub otherwise_parallel_invokes: Vec<InvocationTarget>,
    pub set_statements: Vec<String>,
    pub otherwise_set_statements: Vec<String>,
    pub append_statements: Vec<String>,
    pub otherwise_append_statements: Vec<String>,
    pub compute_statements: Vec<String>,
    pub otherwise_compute_statements: Vec<String>,
    pub delete_statements: Vec<String>,
    pub otherwise_delete_statements: Vec<String>,
    pub iterate_over: Option<String>,
    pub iteration_item: Option<String>,
    pub outputs: BTreeMap<String, OutputValue>,
    pub reads: Vec<String>,
    pub changes: Vec<String>,
    pub external_calls: Vec<String>,
    pub may_fail: Vec<String>,
    pub compensation: Option<String>,
    pub ignored_failures: Vec<String>,
    pub raw_lines: Vec<String>,
}

impl Step {
    pub fn new(number: usize, title: impl Into<String>) -> Self {
        Self {
            number,
            title: title.into(),
            guard: None,
            repeat_while: None,
            repeat_until: None,
            call: None,
            otherwise_call: None,
            invoke: None,
            otherwise_invoke: None,
            parallel_invokes: Vec::new(),
            otherwise_parallel_invokes: Vec::new(),
            set_statements: Vec::new(),
            otherwise_set_statements: Vec::new(),
            append_statements: Vec::new(),
            otherwise_append_statements: Vec::new(),
            compute_statements: Vec::new(),
            otherwise_compute_statements: Vec::new(),
            delete_statements: Vec::new(),
            otherwise_delete_statements: Vec::new(),
            iterate_over: None,
            iteration_item: None,
            outputs: BTreeMap::new(),
            reads: Vec::new(),
            changes: Vec::new(),
            external_calls: Vec::new(),
            may_fail: Vec::new(),
            compensation: None,
            ignored_failures: Vec::new(),
            raw_lines: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureCase {
    pub condition: String,
    pub actions: Vec<String>,
    pub stop_failure: Option<String>,
    pub ignored_failures: Vec<String>,
}

impl FailureCase {
    pub fn new(condition: impl Into<String>) -> Self {
        Self {
            condition: condition.into(),
            actions: Vec::new(),
            stop_failure: None,
            ignored_failures: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guarantee {
    pub conditions: Vec<String>,
    pub statements: Vec<String>,
}

impl Guarantee {
    pub fn new(condition: impl Into<String>) -> Self {
        Self {
            conditions: vec![condition.into()],
            statements: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedQuestion {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    pub name: String,
    pub subjects: BTreeMap<String, Thing>,
    pub inputs: BTreeMap<String, Thing>,
    pub requires: Vec<Requirement>,
    pub state_transitions: Vec<StateTransition>,
    pub steps: Vec<Step>,
    pub failure_handlers: Vec<FailureCase>,
    pub guarantees: Vec<Guarantee>,
    pub unresolved_questions: Vec<UnresolvedQuestion>,
    pub returns: Vec<ReturnValue>,
    pub step_schedule: String,
}

impl Intent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            subjects: BTreeMap::new(),
            inputs: BTreeMap::new(),
            requires: Vec::new(),
            state_transitions: Vec::new(),
            steps: Vec::new(),
            failure_handlers: Vec::new(),
            guarantees: Vec::new(),
            unresolved_questions: Vec::new(),
            returns: Vec::new(),
            step_schedule: "sequential".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub intents: Vec<Intent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RifDocument {
    pub intent: Intent,
    pub intents: Vec<Intent>,
    pub application: Application,
    pub source_path: Option<String>,
}
