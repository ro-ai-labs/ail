//! Closed, nominal schemas for executable variant values.
use super::*;

pub type AilValueSchemas = BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>;

fn error(message: impl Display) -> String {
    format!("AIL-VALUE-001: {message}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TypeExpr {
    pub head: String,
    pub args: Vec<TypeExpr>,
}
impl Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.head)?;
        if !self.args.is_empty() {
            write!(f, "<")?;
            for (i, a) in self.args.iter().enumerate() {
                if i != 0 {
                    write!(f, ",")?;
                }
                write!(f, "{a}")?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}
pub(super) fn identifier(s: &str) -> bool {
    s.len() <= 128 && value_protocol::identifier(s)
}
pub(super) fn qualified(s: &str) -> bool {
    !s.is_empty() && s.split('.').all(identifier)
}
pub(super) fn split_types(s: &str) -> Result<Vec<&str>, String> {
    let mut depth = 0usize;
    let mut start = 0;
    let mut out = Vec::new();
    for (i, b) in s.bytes().enumerate() {
        match b {
            b'<' => {
                depth += 1;
                if depth > 32 {
                    return Err(error("type nesting exceeds 32"));
                }
            }
            b'>' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| error("unbalanced type arguments"))?;
            }
            b',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(error("unbalanced type arguments"));
    }
    out.push(s[start..].trim());
    if out.iter().any(|v| v.is_empty()) {
        return Err(error("empty type argument"));
    }
    Ok(out)
}
pub(super) fn parse(input: &str) -> Result<TypeExpr, String> {
    fn nested(s: &str, depth: usize) -> Result<TypeExpr, String> {
        if depth > 32 {
            return Err(error("type nesting exceeds 32"));
        }
        let s = s.trim();
        let (head, args) = if let Some((head, tail)) = s.split_once('<') {
            let inner = tail
                .strip_suffix('>')
                .ok_or_else(|| error("unbalanced type arguments"))?;
            (
                head.trim(),
                split_types(inner)?
                    .into_iter()
                    .map(|v| nested(v, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        } else {
            (s, Vec::new())
        };
        if !qualified(head) {
            return Err(error("invalid type name"));
        }
        Ok(TypeExpr {
            head: head.into(),
            args,
        })
    }
    if input.len() > 4096 {
        return Err(error("type name exceeds 4096 bytes"));
    }
    nested(input, 1)
}
pub(super) fn supported(schemas: &AilValueSchemas, kind: &str) -> bool {
    value_protocol::supported(kind) || schemas.contains_key(kind)
}
pub(super) fn qualify(
    kind: &str,
    alias: &str,
    declarations: &BTreeSet<String>,
    bound: &BTreeSet<String>,
) -> String {
    fn visit(t: &mut TypeExpr, a: &str, d: &BTreeSet<String>, b: &BTreeSet<String>) {
        if value_protocol::supported(&t.to_string()) {
            return;
        }
        if d.contains(&t.head) && !b.contains(&t.head) {
            t.head = format!("{a}.{}", t.head);
        }
        for arg in &mut t.args {
            visit(arg, a, d, b);
        }
    }
    match parse(kind) {
        Ok(mut t) => {
            visit(&mut t, alias, declarations, bound);
            t.to_string()
        }
        Err(_) => kind.into(),
    }
}
pub(super) fn known(kind: &str, names: &BTreeSet<&str>) -> bool {
    let Ok(t) = parse(kind) else { return false };
    names
        .iter()
        .any(|n| parse(n).is_ok_and(|d| d.head == t.head && d.args.len() == t.args.len()))
}

pub(super) fn from_document(document: &AilDocument) -> Result<AilValueSchemas, String> {
    let mut roots = BTreeSet::new();
    for f in document.functions.values().filter(|f| f.integer.is_some()) {
        roots.extend(
            f.inputs
                .values()
                .chain(f.outputs.values())
                .map(|v| v.type_name.clone()),
        );
        let b = f.integer.as_ref().unwrap();
        roots.extend(b.locals.values().cloned());
        for s in &b.body {
            if let IntegerStep::ValueOperation(ValueOperation::Variant(op)) = s {
                roots.insert(op.type_name().into());
            }
        }
    }
    for a in document.actions.values() {
        for op in &a.value_operations {
            roots.insert(op.type_name().into());
        }
    }
    instantiate(&document.types, roots)
}
fn instantiate(
    types: &BTreeMap<String, AilType>,
    roots: BTreeSet<String>,
) -> Result<AilValueSchemas, String> {
    let mut declarations = BTreeMap::<String, Vec<(&AilType, Vec<String>)>>::new();
    for decl in types.values() {
        // Historical descriptive names are checked only if executable roots use them.
        let Ok(t) = parse(&decl.name) else { continue };
        let params = t.args.iter().map(|a| a.head.clone()).collect::<Vec<_>>();
        declarations.entry(t.head).or_default().push((decl, params));
    }
    let mut out = AilValueSchemas::new();
    let mut pending = roots;
    while let Some(name) = pending.pop_first() {
        if value_protocol::supported(&name) || out.contains_key(&name) {
            continue;
        }
        let t = parse(&name)?;
        if t.to_string() != name {
            return Err(error("executable type spelling must be canonical"));
        }
        let candidates = declarations
            .get(&t.head)
            .ok_or_else(|| error("unknown executable type"))?;
        let [(decl, params)] = candidates.as_slice() else {
            return Err(error("ambiguous type declaration"));
        };
        let declaration = parse(&decl.name)?;
        if params.len() != t.args.len()
            || declaration
                .args
                .iter()
                .any(|a| !a.args.is_empty() || !identifier(&a.head))
            || params.iter().collect::<BTreeSet<_>>().len() != params.len()
        {
            return Err(error("invalid generic declaration or arity"));
        }
        let substitution = params
            .iter()
            .cloned()
            .zip(t.args.clone())
            .collect::<BTreeMap<_, _>>();
        pending.extend(t.args.iter().map(ToString::to_string));
        fn substitute(t: &TypeExpr, vars: &BTreeMap<String, TypeExpr>) -> TypeExpr {
            if t.args.is_empty()
                && let Some(v) = vars.get(&t.head)
            {
                return v.clone();
            }
            TypeExpr {
                head: t.head.clone(),
                args: t.args.iter().map(|a| substitute(a, vars)).collect(),
            }
        }
        let mut variants = BTreeMap::new();
        for (name, v) in &decl.variants {
            if name != &v.name || !identifier(name) {
                return Err(error("invalid variant identity"));
            }
            let mut fields = BTreeMap::new();
            for (name, f) in &v.fields {
                if name != &f.name || !identifier(name) {
                    return Err(error("invalid payload identity"));
                }
                let kind = substitute(&parse(&f.type_name)?, &substitution).to_string();
                parse(&kind)?;
                pending.insert(kind.clone());
                fields.insert(name.clone(), kind);
            }
            variants.insert(name.clone(), fields);
        }
        if variants.is_empty() || variants.len() > 256 || variants.values().any(|v| v.len() > 256) {
            return Err(error(
                "type requires 1 to 256 variants and at most 256 fields per variant",
            ));
        }
        out.insert(t.to_string(), variants);
        if out.len() > 4096
            || out.values().map(BTreeMap::len).sum::<usize>() > 65536
            || out
                .values()
                .flat_map(BTreeMap::values)
                .map(BTreeMap::len)
                .sum::<usize>()
                > 65536
        {
            return Err(error("schema expansion exceeds limits"));
        }
    }
    encode(&out)?;
    Ok(out)
}

pub(super) fn type_code(schemas: &AilValueSchemas, kind: &str) -> Result<u32, String> {
    let builtins = [
        "Int",
        "Float",
        "Bool",
        "Text",
        "Bytes",
        "List<Int>",
        "List<Float>",
        "List<Text>",
        "Json",
    ];
    if let Some(i) = builtins.iter().position(|v| *v == kind) {
        return Ok(i as u32);
    }
    schemas
        .keys()
        .position(|v| v == kind)
        .map(|i| 9 + i as u32)
        .ok_or_else(|| error("unknown closed field type"))
}
pub(super) fn indices(
    schemas: &AilValueSchemas,
    kind: &str,
    variant: &str,
    field: Option<&str>,
) -> Result<(usize, usize, Option<usize>), String> {
    let ti = schemas
        .keys()
        .position(|v| v == kind)
        .ok_or_else(|| error("unknown custom type"))?;
    let variants = &schemas[kind];
    let vi = variants
        .keys()
        .position(|v| v == variant)
        .ok_or_else(|| error("unknown variant"))?;
    let fi = field
        .map(|f| {
            variants[variant]
                .keys()
                .position(|v| v == f)
                .ok_or_else(|| error("unknown variant field"))
        })
        .transpose()?;
    Ok((ti, vi, fi))
}
pub(super) fn encode(schemas: &AilValueSchemas) -> Result<Vec<u8>, String> {
    let variants = schemas.values().map(BTreeMap::len).sum::<usize>();
    let fields = schemas
        .values()
        .flat_map(BTreeMap::values)
        .map(BTreeMap::len)
        .sum::<usize>();
    if schemas.len() > 4096 || variants > 65536 || fields > 65536 {
        return Err(error("schema exceeds count limits"));
    }
    let mut types = Vec::new();
    let mut vs = Vec::new();
    let mut fs = Vec::new();
    let mut names = Vec::new();
    let mut vi = 0u32;
    let mut fi = 0u32;
    for (kind, variants) in schemas {
        let parsed = parse(kind)?;
        fn closed(t: &TypeExpr, schemas: &AilValueSchemas) -> bool {
            t.args
                .iter()
                .all(|a| supported(schemas, &a.to_string()) && closed(a, schemas))
        }
        if !closed(&parsed, schemas)
            || parsed.to_string() != *kind
            || value_protocol::supported(kind)
            || variants.is_empty()
            || variants.len() > 256
        {
            return Err(error("invalid closed type schema"));
        }
        types.extend_from_slice(&vi.to_le_bytes());
        types.extend_from_slice(&(variants.len() as u32).to_le_bytes());
        for (name, fields) in variants {
            if !identifier(name) || fields.len() > 256 {
                return Err(error("invalid variant schema"));
            }
            vs.extend_from_slice(&(names.len() as u32).to_le_bytes());
            vs.extend_from_slice(&(name.len() as u32).to_le_bytes());
            vs.extend_from_slice(&fi.to_le_bytes());
            vs.extend_from_slice(&(fields.len() as u32).to_le_bytes());
            names.extend_from_slice(name.as_bytes());
            for (name, kind) in fields {
                if !identifier(name) {
                    return Err(error("invalid payload name"));
                }
                fs.extend_from_slice(&type_code(schemas, kind)?.to_le_bytes());
                fi += 1;
            }
            vi += 1;
        }
    }
    let mut out = b"AILADT01".to_vec();
    for n in [schemas.len(), variants, fields, names.len()] {
        out.extend_from_slice(&(n as u32).to_le_bytes())
    }
    out.extend_from_slice(&0u64.to_le_bytes());
    out.extend(types);
    out.extend(vs);
    out.extend(fs);
    out.extend(names);
    if out.len() > 4 * 1024 * 1024 {
        return Err(error("encoded schema exceeds 4 MiB"));
    }
    // The engine independently checks the dense table ownership and code ranges.
    if unsafe { crate::value_engine::schema_check(out.as_ptr(), out.len() as u64) } != 0 {
        return Err(error("invalid encoded schema"));
    }
    Ok(out)
}
pub(super) fn render(schemas: &AilValueSchemas) -> String {
    format!(
        "{{{}}}",
        schemas
            .iter()
            .map(|(kind, variants)| format!(
                "{}:{{{}}}",
                json_string(kind),
                variants
                    .iter()
                    .map(|(name, fields)| format!(
                        "{}:{}",
                        json_string(name),
                        render_json_string_map(fields)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}
pub(super) fn parse_metadata(
    root: &BTreeMap<String, AilJsonValue>,
) -> Result<AilValueSchemas, String> {
    let Some(value) = root.get("type_schemas") else {
        return Ok(AilValueSchemas::new());
    };
    let object = value
        .as_object()
        .ok_or_else(|| error("schemas must be an object"))?;
    let mut out = AilValueSchemas::new();
    for (kind, variants) in object {
        let mut vs = BTreeMap::new();
        for (name, fields) in variants
            .as_object()
            .ok_or_else(|| error("variants must be an object"))?
        {
            let mut fs = BTreeMap::new();
            for (name, kind) in fields
                .as_object()
                .ok_or_else(|| error("fields must be an object"))?
            {
                fs.insert(
                    name.clone(),
                    kind.as_string()
                        .ok_or_else(|| error("field type must be a string"))?
                        .into(),
                );
            }
            vs.insert(name.clone(), fs);
        }
        out.insert(kind.clone(), vs);
    }
    encode(&out)?;
    Ok(out)
}

/// Validate declarations before a Core projection can erase duplicate ownership.
pub(super) fn from_core(core: &AilCore) -> Result<AilValueSchemas, String> {
    let mut types = BTreeMap::new();
    for ty in core.graph.nodes.iter().filter(|n| n.kind == "Type") {
        let mut decl = AilType {
            name: ty.name.clone(),
            label: ty.attributes.get("label").cloned().unwrap_or_default(),
            ..AilType::default()
        };
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|e| e.source == ty.id && e.kind == "contains")
        {
            let Some(v) = core
                .graph
                .nodes
                .iter()
                .find(|n| n.id == edge.target && n.kind == "Variant")
            else {
                continue;
            };
            let name = v
                .name
                .strip_prefix(&format!("{}.", ty.name))
                .unwrap_or(&v.name)
                .to_string();
            let mut variant = AilVariant {
                name: name.clone(),
                label: v.attributes.get("label").cloned().unwrap_or_default(),
                ..AilVariant::default()
            };
            for e in core
                .graph
                .edges
                .iter()
                .filter(|e| e.source == v.id && e.kind == "has_field")
            {
                let Some(f) = core
                    .graph
                    .nodes
                    .iter()
                    .find(|n| n.id == e.target && n.kind == "Field")
                else {
                    continue;
                };
                let name = f
                    .name
                    .strip_prefix(&format!("{}.", v.name))
                    .unwrap_or(&f.name)
                    .to_string();
                variant.fields.insert(
                    name.clone(),
                    AilVariantField {
                        name,
                        type_name: f.type_name.clone().unwrap_or_default(),
                        provenance: String::new(),
                    },
                );
            }
            decl.variants.insert(name, variant);
        }
        types.insert(decl.name.clone(), decl);
    }
    let mut roots = BTreeSet::new();
    for f in core.graph.nodes.iter().filter(|n| {
        n.kind == "Function"
            && (n.attributes.contains_key("executable_value")
                || n.attributes.contains_key("executable_integer"))
    }) {
        for e in core.graph.edges.iter().filter(|e| {
            e.source == f.id
                && matches!(
                    e.kind.as_str(),
                    "has_input" | "has_output" | "value_local" | "value_parameter"
                )
        }) {
            if let Some(t) = core
                .graph
                .nodes
                .iter()
                .find(|n| n.id == e.target)
                .and_then(|n| n.type_name.clone())
            {
                roots.insert(t);
            }
        }
    }
    for n in &core.graph.nodes {
        if (n.kind == "VariantOperation"
            || n.attributes.get("opcode").is_some_and(|o| {
                matches!(o.as_str(), "VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET")
            }))
            && let Some(t) = n.attributes.get("type")
        {
            roots.insert(t.clone());
        }
    }
    let schemas = instantiate(&types, roots)?;
    let heads = schemas
        .keys()
        .map(|n| parse(n).map(|t| t.head))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut names = BTreeSet::new();
    for ty in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "Type" && parse(&n.name).is_ok_and(|t| heads.contains(&t.head)))
    {
        if !names.insert(&ty.name)
            || ty.type_name.is_some()
            || ty.attributes.keys().map(String::as_str).collect::<Vec<_>>() != ["label"]
        {
            return Err(error("invalid reachable Type identity or attributes"));
        }
        check_provenance(core, ty)?;
        let mut variants = BTreeSet::new();
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|e| e.source == ty.id && e.kind != "has_provenance")
        {
            let v = core
                .graph
                .nodes
                .iter()
                .find(|n| n.id == edge.target)
                .ok_or_else(|| error("missing variant"))?;
            let name = v
                .name
                .strip_prefix(&format!("{}.", ty.name))
                .ok_or_else(|| error("foreign variant"))?;
            if edge.kind != "contains"
                || !edge.attributes.is_empty()
                || v.kind != "Variant"
                || !identifier(name)
                || !variants.insert(name)
                || v.type_name.is_some()
                || v.attributes.keys().map(String::as_str).collect::<Vec<_>>() != ["label"]
                || core
                    .graph
                    .edges
                    .iter()
                    .filter(|e| e.target == v.id && e.kind == "contains")
                    .count()
                    != 1
            {
                return Err(error("invalid variant ownership or attributes"));
            }
            check_provenance(core, v)?;
            let mut fields = BTreeSet::new();
            for e in core
                .graph
                .edges
                .iter()
                .filter(|e| e.source == v.id && e.kind != "has_provenance")
            {
                let f = core
                    .graph
                    .nodes
                    .iter()
                    .find(|n| n.id == e.target)
                    .ok_or_else(|| error("missing payload"))?;
                let name = f
                    .name
                    .strip_prefix(&format!("{}.", v.name))
                    .ok_or_else(|| error("foreign payload"))?;
                if e.kind != "has_field"
                    || !e.attributes.is_empty()
                    || f.kind != "Field"
                    || !identifier(name)
                    || !fields.insert(name)
                    || !f.attributes.is_empty()
                    || f.type_name.is_none()
                    || core
                        .graph
                        .edges
                        .iter()
                        .filter(|e| e.target == f.id && e.kind == "has_field")
                        .count()
                        != 1
                {
                    return Err(error("invalid payload ownership or attributes"));
                }
                check_provenance(core, f)?;
            }
        }
    }
    Ok(schemas)
}
pub(super) fn check_provenance(core: &AilCore, node: &Node) -> Result<(), String> {
    let edges = core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == "has_provenance")
        .collect::<Vec<_>>();
    if edges.len() != 1
        || !edges[0].attributes.is_empty()
        || !core
            .graph
            .nodes
            .iter()
            .any(|n| n.id == edges[0].target && n.kind == "Provenance")
    {
        return Err(error("declaration requires one provenance"));
    }
    Ok(())
}
