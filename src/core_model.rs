use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub type_name: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

impl Node {
    pub fn new(
        kind: impl Into<String>,
        name: impl Into<String>,
        type_name: Option<String>,
        attributes: BTreeMap<String, String>,
    ) -> Self {
        let kind = kind.into();
        let name = name.into();
        let id = stable_id(&kind, &name, &attributes);
        Self {
            id,
            kind,
            name,
            type_name,
            attributes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub target: String,
    pub attributes: BTreeMap<String, String>,
}

impl Edge {
    pub fn new(
        kind: impl Into<String>,
        source: &Node,
        target: &Node,
        attributes: BTreeMap<String, String>,
    ) -> Self {
        let kind = kind.into();
        let basis = format!("{}:{}:{}:{attributes:?}", kind, source.id, target.id);
        Self {
            id: format!("edge:{}:{:016x}", slug(&kind), stable_hash(&basis)),
            kind,
            source: source.id.clone(),
            target: target.id.clone(),
            attributes,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn add_node(
        &mut self,
        kind: impl Into<String>,
        name: impl Into<String>,
        type_name: Option<String>,
        attributes: BTreeMap<String, String>,
    ) -> Node {
        let kind = kind.into();
        let name = name.into();
        if let Some(existing) = self
            .nodes
            .iter()
            .find(|node| node.kind == kind && node.name == name)
        {
            return existing.clone();
        }
        let node = Node::new(kind, name, type_name, attributes);
        self.nodes.push(node.clone());
        node
    }

    pub fn add_edge(
        &mut self,
        kind: impl Into<String>,
        source: &Node,
        target: &Node,
        attributes: BTreeMap<String, String>,
    ) -> Edge {
        let kind = kind.into();
        if let Some(existing) = self
            .edges
            .iter()
            .find(|edge| edge.kind == kind && edge.source == source.id && edge.target == target.id)
        {
            return existing.clone();
        }
        let edge = Edge::new(kind, source, target, attributes);
        self.edges.push(edge.clone());
        edge
    }

    pub fn find_node(&self, kind: &str, name: &str) -> Option<&Node> {
        self.nodes
            .iter()
            .find(|node| node.kind == kind && node.name == name)
    }

    pub fn has_edge(&self, kind: &str, source_name: &str, target_name: &str) -> bool {
        let source_ids: BTreeSet<_> = self
            .nodes
            .iter()
            .filter(|node| node.name == source_name)
            .map(|node| node.id.as_str())
            .collect();
        let target_ids: BTreeSet<_> = self
            .nodes
            .iter()
            .filter(|node| node.name == target_name)
            .map(|node| node.id.as_str())
            .collect();
        self.edges.iter().any(|edge| {
            edge.kind == kind
                && source_ids.contains(edge.source.as_str())
                && target_ids.contains(edge.target.as_str())
        })
    }

    pub fn validate_edge_references(&self) -> Vec<String> {
        let node_ids: BTreeSet<_> = self.nodes.iter().map(|node| node.id.as_str()).collect();
        let mut missing = Vec::new();
        for edge in &self.edges {
            if !node_ids.contains(edge.source.as_str()) {
                missing.push(format!("{} missing source {}", edge.id, edge.source));
            }
            if !node_ids.contains(edge.target.as_str()) {
                missing.push(format!("{} missing target {}", edge.id, edge.target));
            }
        }
        missing
    }

    pub fn node_ids(&self) -> Vec<String> {
        self.nodes.iter().map(|node| node.id.clone()).collect()
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"nodes\":[{}],\"edges\":[{}]}}",
            self.nodes
                .iter()
                .map(node_json)
                .collect::<Vec<_>>()
                .join(","),
            self.edges
                .iter()
                .map(edge_json)
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreModule {
    pub name: String,
    pub intents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreOperation {
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequirement {
    pub kind: String,
    pub target: String,
}

impl PermissionRequirement {
    pub fn name(&self) -> String {
        format!("{} {}", self.kind, self.target)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreEffect {
    pub kind: String,
    pub target: String,
}

impl CoreEffect {
    pub fn name(&self) -> String {
        format!("{} {}", self.kind, self.target)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub modules: Vec<CoreModule>,
    pub graph: Graph,
    pub operations: Vec<CoreOperation>,
    pub permissions: Vec<PermissionRequirement>,
    pub effects: Vec<CoreEffect>,
    pub views: Vec<View>,
    pub metadata: BTreeMap<String, String>,
}

impl Program {
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"modules\":[{}],",
                "\"graph\":{},",
                "\"operations\":[{}],",
                "\"permissions\":[{}],",
                "\"effects\":[{}],",
                "\"views\":[{}],",
                "\"metadata\":{}",
                "}}"
            ),
            self.modules
                .iter()
                .map(module_json)
                .collect::<Vec<_>>()
                .join(","),
            self.graph.to_json(),
            self.operations
                .iter()
                .map(operation_json)
                .collect::<Vec<_>>()
                .join(","),
            self.permissions
                .iter()
                .map(permission_json)
                .collect::<Vec<_>>()
                .join(","),
            self.effects
                .iter()
                .map(effect_json)
                .collect::<Vec<_>>()
                .join(","),
            self.views
                .iter()
                .map(view_json)
                .collect::<Vec<_>>()
                .join(","),
            string_map_json(&self.metadata)
        )
    }
}

pub fn attr(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn stable_id(kind: &str, name: &str, attributes: &BTreeMap<String, String>) -> String {
    let basis = format!("{kind}:{name}:{attributes:?}");
    format!("{}:{}:{:016x}", kind, slug(name), stable_hash(&basis))
}

fn stable_hash(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn slug(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "unnamed".to_string()
    } else {
        out
    }
}

fn module_json(module: &CoreModule) -> String {
    format!(
        "{{\"name\":{},\"intents\":[{}]}}",
        json_string(&module.name),
        module
            .intents
            .iter()
            .map(|intent| json_string(intent))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn node_json(node: &Node) -> String {
    let type_part = node
        .type_name
        .as_ref()
        .map(|type_name| format!(",\"type\":{}", json_string(type_name)))
        .unwrap_or_default();
    format!(
        "{{\"id\":{},\"kind\":{},\"name\":{}{},\"attributes\":{}}}",
        json_string(&node.id),
        json_string(&node.kind),
        json_string(&node.name),
        type_part,
        string_map_json(&node.attributes)
    )
}

fn edge_json(edge: &Edge) -> String {
    format!(
        "{{\"id\":{},\"kind\":{},\"source\":{},\"target\":{},\"attributes\":{}}}",
        json_string(&edge.id),
        json_string(&edge.kind),
        json_string(&edge.source),
        json_string(&edge.target),
        string_map_json(&edge.attributes)
    )
}

fn operation_json(operation: &CoreOperation) -> String {
    format!(
        "{{\"name\":{},\"inputs\":[{}],\"outputs\":[{}]}}",
        json_string(&operation.name),
        operation
            .inputs
            .iter()
            .map(|value| json_string(value))
            .collect::<Vec<_>>()
            .join(","),
        operation
            .outputs
            .iter()
            .map(|value| json_string(value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn permission_json(permission: &PermissionRequirement) -> String {
    format!(
        "{{\"kind\":{},\"target\":{},\"name\":{}}}",
        json_string(&permission.kind),
        json_string(&permission.target),
        json_string(&permission.name())
    )
}

fn effect_json(effect: &CoreEffect) -> String {
    format!(
        "{{\"kind\":{},\"target\":{},\"name\":{}}}",
        json_string(&effect.kind),
        json_string(&effect.target),
        json_string(&effect.name())
    )
}

fn view_json(view: &View) -> String {
    format!(
        "{{\"name\":{},\"content\":{}}}",
        json_string(&view.name),
        json_string(&view.content)
    )
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

pub fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

pub fn json_value(value: &str) -> String {
    let value = value.trim();
    if matches!(value, "true" | "false" | "null") || value == "None" {
        return if value == "None" {
            "null".to_string()
        } else {
            value.to_string()
        };
    }
    if is_json_number(value) || is_json_container(value) {
        return value.to_string();
    }
    if let Some(inner) = value
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
    {
        return json_string(inner);
    }
    json_string(value)
}

fn is_json_number(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut index = usize::from(bytes[0] == b'-');
    if index >= bytes.len() {
        return false;
    }
    if bytes[index] == b'0' {
        index += 1;
    } else if bytes[index].is_ascii_digit() {
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
    } else {
        return false;
    }
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        let decimal_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == decimal_start {
            return false;
        }
    }
    index == bytes.len()
}

fn is_json_container(value: &str) -> bool {
    (value.starts_with('{') && value.ends_with('}'))
        || (value.starts_with('[') && value.ends_with(']'))
}
