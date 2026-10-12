//! Shared allocation-free executable variant engine.
#[repr(C)]
pub struct ValueSpan {
    pub data: *const u8,
    pub len: u64,
}
#[repr(C)]
pub struct ValueRequest {
    pub kind: u64,
    pub schema: *const u8,
    pub schema_len: u64,
    pub type_index: u64,
    pub variant_index: u64,
    pub field_index: u64,
    pub input: *const u8,
    pub input_len: u64,
    pub args: *const ValueSpan,
    pub arg_count: u64,
    pub output: *mut u8,
    pub output_capacity: u64,
    pub scratch: *mut u8,
    pub scratch_capacity: u64,
}
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct ValueResult {
    pub output_len: u64,
    pub scalar: u64,
    pub result_kind: u64,
}

use crate::value_transport::{self as t, Count, Cursor, Output, Quoted, Result, Sink};
pub const MAX_BYTES: usize = 8_388_608;
pub const SCRATCH_BYTES: usize = MAX_BYTES;
struct Schema<'a> {
    bytes: &'a [u8],
    types: usize,
    variants: usize,
    fields: usize,
    variant_start: usize,
    field_start: usize,
    names_start: usize,
}
impl<'a> Schema<'a> {
    fn n(&self, at: usize) -> Result<usize> {
        Ok(u32::from_le_bytes(
            self.bytes
                .get(at..at + 4)
                .ok_or(3u64)?
                .try_into()
                .map_err(|_| 3u64)?,
        ) as usize)
    }
    fn new(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() > 4_194_304
            || bytes.get(..8) != Some(b"AILADT01")
            || bytes.get(24..32) != Some(&[0; 8])
        {
            return Err(3);
        }
        let mut s = Self {
            bytes,
            types: 0,
            variants: 0,
            fields: 0,
            variant_start: 0,
            field_start: 0,
            names_start: 0,
        };
        s.types = s.n(8)?;
        s.variants = s.n(12)?;
        s.fields = s.n(16)?;
        if s.types > 4096 || s.variants > 65_536 || s.fields > 65_536 {
            return Err(3);
        }
        s.variant_start = 32 + s.types * 8;
        s.field_start = s.variant_start + s.variants * 16;
        s.names_start = s.field_start + s.fields * 4;
        if s.names_start.checked_add(s.n(20)?) != Some(bytes.len()) {
            return Err(3);
        }
        Ok(s)
    }
    fn type_range(&self, index: usize) -> Result<(usize, usize)> {
        if index >= self.types {
            return Err(3);
        }
        let start = self.n(32 + index * 8)?;
        let count = self.n(36 + index * 8)?;
        if count > 256 || start + count > self.variants {
            return Err(3);
        }
        Ok((start, count))
    }
    fn variant(&self, kind: usize, index: usize) -> Result<(usize, &'a [u8], usize, usize)> {
        let (start, count) = self.type_range(kind)?;
        if index >= count {
            return Err(3);
        }
        let at = self.variant_start + (start + index) * 16;
        let name = self.n(at)?;
        let len = self.n(at + 4)?;
        let first = self.n(at + 8)?;
        let count = self.n(at + 12)?;
        if count > 256 || first + count > self.fields {
            return Err(3);
        }
        let bytes = self
            .bytes
            .get(self.names_start + name..self.names_start + name + len)
            .ok_or(3u64)?;
        Ok((at, bytes, first, count))
    }
    fn field(&self, first: usize, index: usize) -> Result<u32> {
        let n = self.n(self.field_start + (first + index) * 4)?;
        if n >= 9 + self.types {
            return Err(3);
        }
        Ok(n as u32)
    }
    fn check(&self) -> Result<()> {
        let (mut variants, mut fields, mut names) = (0, 0, 0);
        for ty in 0..self.types {
            let (first, count) = self.type_range(ty)?;
            if first != variants {
                return Err(3);
            }
            let mut previous: &[u8] = &[];
            for v in 0..count {
                let (at, name, first, count) = self.variant(ty, v)?;
                if name.is_empty()
                    || name.len() > 128
                    || !name[0].is_ascii_alphabetic() && name[0] != b'_'
                    || !name.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    || name <= previous
                    || first != fields
                    || self.n(at)? != names
                {
                    return Err(3);
                }
                for f in 0..count {
                    self.field(first, f)?;
                }
                fields += count;
                names += name.len();
                previous = name;
            }
            variants += count;
        }
        if variants != self.variants
            || fields != self.fields
            || self.names_start + names != self.bytes.len()
        {
            return Err(3);
        }
        Ok(())
    }
}
unsafe fn bytes<'a>(p: *const u8, len: u64, bound: usize, status: u64) -> Result<&'a [u8]> {
    if len > bound as u64 {
        return Err(status);
    }
    if p.is_null() {
        return if len == 0 { Ok(&[]) } else { Err(1) };
    }
    Ok(unsafe { core::slice::from_raw_parts(p, len as usize) })
}
/// # Safety
/// The schema pointer must address its declared readable bytes.
pub unsafe fn schema_check(schema: *const u8, len: u64) -> u64 {
    let result = (|| {
        let b = unsafe { bytes(schema, len, 4_194_304, 3)? };
        Schema::new(b)?.check()
    })();
    result.err().unwrap_or(0)
}
struct Walker<'a, 's> {
    schema: &'a Schema<'a>,
    scratch: &'s mut [u8],
    nodes: usize,
    owned: bool,
    selected: Option<(usize, usize, u32)>,
    select_field: Option<usize>,
}
impl Walker<'_, '_> {
    fn leaf(
        &mut self,
        c: &mut Cursor<'_>,
        kind: u32,
        out: &mut dyn Sink,
        quote: bool,
    ) -> Result<u64> {
        let mut decoded = Output {
            bytes: self.scratch,
            len: 0,
        };
        c.string(&mut decoded)?;
        let value = &decoded.bytes[..decoded.len];
        let (_, scalar) = crate::value_leaf::measure(kind, value, self.owned)?;
        if quote {
            out.put(b"\"")?;
            crate::value_leaf::write(kind, value, &mut Quoted(out))?;
            out.put(b"\"")?;
        } else {
            crate::value_leaf::write(kind, value, out)?;
        }
        Ok(scalar)
    }
    fn value(
        &mut self,
        c: &mut Cursor<'_>,
        kind: u32,
        depth: usize,
        out: &mut dyn Sink,
    ) -> Result<usize> {
        self.nodes += 1;
        if self.nodes > 65_536 {
            return Err(2);
        }
        c.space();
        if kind < 9 {
            self.leaf(c, kind, out, true)?;
            return Ok(0);
        }
        if depth > 64 {
            return Err(2);
        }
        let ty = (kind - 9) as usize;
        c.need(b'[')?;
        c.space();
        let mut name = Output {
            bytes: self.scratch,
            len: 0,
        };
        c.string(&mut name)?;
        let tag = &name.bytes[..name.len];
        let (_, count) = self.schema.type_range(ty)?;
        let mut selected = None;
        for index in 0..count {
            if self.schema.variant(ty, index)?.1 == tag {
                selected = Some(index);
                break;
            }
        }
        let variant = selected.ok_or(1u64)?;
        let (_, tag, first, count) = self.schema.variant(ty, variant)?;
        out.put(b"[")?;
        t::quote(tag, out)?;
        for field in 0..count {
            c.space();
            c.need(b',')?;
            c.space();
            out.put(b",")?;
            let start = c.at;
            let kind = self.schema.field(first, field)?;
            self.value(c, kind, if kind >= 9 { depth + 1 } else { depth }, out)?;
            if depth == 1 && self.select_field == Some(field) {
                self.selected = Some((start, c.at, kind));
            }
        }
        c.space();
        c.need(b']')?;
        out.put(b"]")?;
        Ok(variant)
    }
}
unsafe fn scratch<'a>(request: &ValueRequest) -> Result<&'a mut [u8]> {
    if request.scratch.is_null()
        || request.scratch_capacity < SCRATCH_BYTES as u64
        || request.scratch_capacity > isize::MAX as u64
    {
        return Err(3);
    }
    Ok(unsafe { core::slice::from_raw_parts_mut(request.scratch, SCRATCH_BYTES) })
}
fn source<'a>(request: &ValueRequest) -> Result<&'a [u8]> {
    if request.input.is_null() {
        return Err(1);
    }
    unsafe { bytes(request.input, request.input_len, MAX_BYTES, 2) }
}
fn validate_value(
    schema: &Schema<'_>,
    kind: u32,
    input: &[u8],
    scratch: &mut [u8],
    owned: bool,
    out: &mut dyn Sink,
) -> Result<u64> {
    if kind < 9 {
        return crate::value_leaf::write(kind, input, out).and_then(|scalar| {
            crate::value_leaf::measure(kind, input, owned)?;
            Ok(scalar)
        });
    }
    if input.len() > MAX_BYTES {
        return Err(2);
    }
    let mut c = Cursor::new(input)?;
    let mut walker = Walker {
        schema,
        scratch,
        nodes: 0,
        owned,
        selected: None,
        select_field: None,
    };
    walker.value(&mut c, kind, 1, out)?;
    c.done()?;
    Ok(0)
}
/// Validate one constructor argument before evaluating the next argument.
/// `type_index` contains its type code; other fields follow ValueRequest.
/// # Safety
/// Request/schema/input/scratch storage must satisfy run's contract.
pub unsafe fn validate_argument(request: *const ValueRequest) -> u64 {
    if request.is_null() {
        return 3;
    }
    let result = (|| {
        let r = unsafe { &*request };
        let schema = Schema::new(unsafe { bytes(r.schema, r.schema_len, 4_194_304, 3)? })?;
        let kind = u32::try_from(r.type_index).map_err(|_| 3u64)?;
        if kind as usize >= 9 + schema.types {
            return Err(3);
        }
        validate_value(
            &schema,
            kind,
            source(r)?,
            unsafe { scratch(r)? },
            false,
            &mut Count(0),
        )?;
        Ok(())
    })();
    result.err().unwrap_or(0)
}
fn construct(
    schema: &Schema<'_>,
    request: &ValueRequest,
    args: &[ValueSpan],
    scratch: &mut [u8],
    out: &mut dyn Sink,
) -> Result<()> {
    let (_, tag, first, count) =
        schema.variant(request.type_index as usize, request.variant_index as usize)?;
    if args.len() != count {
        return Err(3);
    }
    let mut walker = Walker {
        schema,
        scratch,
        nodes: 1,
        owned: true,
        selected: None,
        select_field: None,
    };
    out.put(b"[")?;
    t::quote(tag, out)?;
    for (index, arg) in args.iter().enumerate() {
        let kind = schema.field(first, index)?;
        if arg.data.is_null() {
            return Err(1);
        }
        let value = unsafe { bytes(arg.data, arg.len, MAX_BYTES, 2)? };
        out.put(b",")?;
        if kind < 9 {
            walker.nodes += 1;
            if walker.nodes > 65_536 {
                return Err(2);
            }
            crate::value_leaf::measure(kind, value, true)?;
            out.put(b"\"")?;
            crate::value_leaf::write(kind, value, &mut Quoted(out))?;
            out.put(b"\"")?;
        } else {
            let mut cursor = Cursor::new(value)?;
            walker.value(&mut cursor, kind, 2, out)?;
            cursor.done()?;
        }
    }
    out.put(b"]")
}
unsafe fn execute(r: &ValueRequest, result: *mut ValueResult) -> Result<()> {
    if r.kind > 4 {
        return Err(3);
    }
    let schema = Schema::new(unsafe { bytes(r.schema, r.schema_len, 4_194_304, 3)? })?;
    schema.type_range(r.type_index as usize)?;
    let scratch = unsafe { scratch(r)? };
    let mut count = Count(0);
    let mut scalar = 0;
    let mut result_kind = r.type_index as u32 + 9;
    let mut selected = None;
    let mut args = &[][..];
    if r.kind == 2 {
        if r.arg_count > 256 || r.arg_count != 0 && r.args.is_null() {
            return Err(3);
        }
        if r.arg_count != 0 {
            args = unsafe { core::slice::from_raw_parts(r.args, r.arg_count as usize) };
        }
        construct(&schema, r, args, scratch, &mut count)?;
    } else {
        let input = source(r)?;
        let mut c = Cursor::new(input)?;
        let mut walker = Walker {
            schema: &schema,
            scratch,
            nodes: 0,
            owned: r.kind == 1,
            selected: None,
            select_field: if r.kind == 4 {
                Some(r.field_index as usize)
            } else {
                None
            },
        };
        let variant = walker.value(&mut c, result_kind, 1, &mut count)?;
        c.done()?;
        if r.kind == 3 || r.kind == 4 {
            let (_, _, _, fields) =
                schema.variant(r.type_index as usize, r.variant_index as usize)?;
            if r.kind == 4 && r.field_index as usize >= fields {
                return Err(3);
            }
            if r.kind == 3 {
                scalar = u64::from(variant == r.variant_index as usize);
                result_kind = 2;
                count.0 = 0;
            } else {
                if variant != r.variant_index as usize {
                    return Err(1);
                }
                selected = walker.selected;
                let (start, end, kind) = selected.ok_or(3u64)?;
                result_kind = kind;
                count.0 = 0;
                let mut c = Cursor::new(&input[start..end])?;
                walker.owned = true;
                if kind < 9 {
                    scalar = walker.leaf(&mut c, kind, &mut count, false)?;
                } else {
                    walker.nodes = 0;
                    walker.value(&mut c, kind, 1, &mut count)?;
                }
                c.done()?;
            }
        }
    }
    if r.kind == 0 || r.kind == 3 {
        unsafe {
            result.write(ValueResult {
                output_len: 0,
                scalar,
                result_kind: result_kind as u64,
            })
        };
        return Ok(());
    }
    if count.0 > MAX_BYTES {
        return Err(2);
    }
    if count.0 as u64 > r.output_capacity || r.output.is_null() {
        return Err(3);
    }
    let mut out = Output {
        bytes: unsafe { core::slice::from_raw_parts_mut(r.output, count.0) },
        len: 0,
    };
    if r.kind == 2 {
        construct(&schema, r, args, scratch, &mut out)?;
    } else {
        let input = source(r)?;
        if let Some((start, end, kind)) = selected {
            let mut c = Cursor::new(&input[start..end])?;
            let mut walker = Walker {
                schema: &schema,
                scratch,
                nodes: 0,
                owned: true,
                selected: None,
                select_field: None,
            };
            if kind < 9 {
                walker.leaf(&mut c, kind, &mut out, false)?;
            } else {
                walker.value(&mut c, kind, 1, &mut out)?;
            }
        } else {
            validate_value(&schema, result_kind, input, scratch, true, &mut out)?;
        }
    }
    unsafe {
        result.write(ValueResult {
            output_len: out.len as u64,
            scalar,
            result_kind: result_kind as u64,
        })
    };
    Ok(())
}
/// Execute with caller-owned scratch and output. Result/output remain untouched on
/// validation errors; scratch is temporary and may be modified on every path.
/// # Safety
/// Non-null pointers must address their declared lengths. Request/result must be
/// aligned; output, scratch, descriptors and readable inputs must be disjoint.
/// The schema must have passed schema_check. All inputs are immutable during call.
pub unsafe fn run(request: *const ValueRequest, result: *mut ValueResult) -> u64 {
    if request.is_null() || result.is_null() {
        return 3;
    }
    unsafe { execute(&*request, result) }.err().unwrap_or(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn schema(fields: &[u32]) -> Vec<u8> {
        let mut b = b"AILADT01".to_vec();
        for n in [
            1u32,
            1,
            fields.len() as u32,
            1,
            0,
            0,
            0,
            1,
            0,
            1,
            0,
            fields.len() as u32,
        ] {
            b.extend_from_slice(&n.to_le_bytes());
        }
        for n in fields {
            b.extend_from_slice(&n.to_le_bytes());
        }
        b.push(b'V');
        b
    }
    fn execute(fields: &[u32], input: &str, kind: u64, field: u64) -> (u64, String, ValueResult) {
        let s = schema(fields);
        let mut scratch = vec![0; 8_388_608];
        let mut output = vec![0; 8_388_608];
        let q = ValueRequest {
            kind,
            schema: s.as_ptr(),
            schema_len: s.len() as u64,
            type_index: 0,
            variant_index: 0,
            field_index: field,
            input: input.as_ptr(),
            input_len: input.len() as u64,
            args: core::ptr::null(),
            arg_count: 0,
            output: output.as_mut_ptr(),
            output_capacity: output.len() as u64,
            scratch: scratch.as_mut_ptr(),
            scratch_capacity: scratch.len() as u64,
        };
        let mut r = ValueResult {
            output_len: 0,
            scalar: 0,
            result_kind: 0,
        };
        let status = unsafe { run(&q, &mut r) };
        (
            status,
            String::from_utf8(output[..r.output_len as usize].to_vec()).unwrap(),
            r,
        )
    }

    fn tree_schema() -> Vec<u8> {
        let mut b = b"AILADT01".to_vec();
        for n in [1u32, 2, 2, 2, 0, 0, 0, 2, 0, 1, 0, 0, 1, 1, 0, 2, 9, 9] {
            b.extend_from_slice(&n.to_le_bytes());
        }
        b.extend_from_slice(b"EN");
        b
    }
    fn run_schema(
        schema: &[u8],
        input: &[u8],
        kind: u64,
        args: &[&[u8]],
    ) -> (u64, ValueResult, Vec<u8>) {
        let mut scratch = vec![0; SCRATCH_BYTES];
        let mut output = vec![0x55; MAX_BYTES];
        let args = args
            .iter()
            .map(|a| ValueSpan {
                data: a.as_ptr(),
                len: a.len() as u64,
            })
            .collect::<Vec<_>>();
        let request = ValueRequest {
            kind,
            schema: schema.as_ptr(),
            schema_len: schema.len() as u64,
            type_index: 0,
            variant_index: 1,
            field_index: 0,
            input: input.as_ptr(),
            input_len: input.len() as u64,
            args: args.as_ptr(),
            arg_count: args.len() as u64,
            output: output.as_mut_ptr(),
            output_capacity: output.len() as u64,
            scratch: scratch.as_mut_ptr(),
            scratch_capacity: scratch.len() as u64,
        };
        let mut result = ValueResult {
            output_len: 777,
            scalar: 888,
            result_kind: 999,
        };
        let status = unsafe { run(&request, &mut result) };
        (status, result, output)
    }
    #[test]
    fn recursive_validation_counts_complete_values_and_checks_constructor_depth() {
        let schema = tree_schema();
        assert_eq!(
            unsafe { schema_check(schema.as_ptr(), schema.len() as u64) },
            0
        );
        let mut chain = "[\"E\"]".to_string();
        for _ in 1..64 {
            chain = format!("[\"N\",[\"E\"],{chain}]");
        }
        assert_eq!(run_schema(&schema, chain.as_bytes(), 0, &[]).0, 0);
        let (status, result, output) = run_schema(&schema, b"", 2, &[b"[\"E\"]", chain.as_bytes()]);
        assert_eq!(status, 2);
        assert_eq!(result.output_len, 777);
        assert_eq!(output[0], 0x55);
        let mut tree = "[\"E\"]".to_string();
        for _ in 0..15 {
            tree = format!("[\"N\",{tree},{tree}]");
        }
        assert_eq!(run_schema(&schema, tree.as_bytes(), 0, &[]).0, 0);
        assert_eq!(
            run_schema(&schema, b"", 2, &[tree.as_bytes(), b"[\"E\"]"]).0,
            2
        );
    }
    #[test]
    fn borrow_can_inspect_canonical_expansion_and_failed_return_is_atomic() {
        let input = format!(
            "[\"V\",\"{}\",\"{}\",\"{}\"]",
            "\u{7f}".repeat(1_048_576),
            "\u{7f}".repeat(1_048_576),
            "\u{7f}".repeat(1_048_576)
        );
        assert_eq!(execute(&[3, 3, 3], &input, 0, 0).0, 0);
        assert_eq!(execute(&[3, 3, 3], &input, 3, 0).0, 0);
        assert_eq!(execute(&[3, 3, 3], &input, 1, 0).0, 2);
        let (status, output, _) = execute(&[3, 3, 3], &input, 4, 0);
        assert_eq!(status, 0);
        assert_eq!(output.len(), 1_048_576);
    }
    #[test]
    fn borrowed_builtin_canonical_expansion_is_checked_only_when_produced() {
        let contents = "\u{85}".repeat(200_000);
        for (kind, leaf) in [
            (8, format!("\\\"{contents}\\\"")),
            (7, format!("[\\\"{contents}\\\"]")),
        ] {
            let input = format!("[\"V\",\"{leaf}\"]");
            for mode in [0, 3] {
                assert_eq!(execute(&[kind], &input, mode, 0).0, 0, "{kind} {mode}");
            }
            for mode in [1, 4] {
                assert_eq!(execute(&[kind], &input, mode, 0).0, 2, "{kind} {mode}");
            }
        }
    }
    #[test]
    fn builtin_limits_and_invalid_string_escapes_are_distinct() {
        for input in [r#"["V","\ud800"]"#, r#"["V","\uDC00"]"#, r#"["V","\x"]"#] {
            assert_eq!(execute(&[3], input, 0, 0).0, 1);
        }
        let input = format!("[\"V\",\"hex:{}\"]", "00".repeat(1_048_577));
        assert_eq!(execute(&[4], &input, 0, 0).0, 2);
        let input = format!("[\"V\",\"[{}]\"]", vec!["0"; 65_537].join(","));
        assert_eq!(execute(&[5], &input, 0, 0).0, 2);
        let input = format!("[\"V\",\"{}\"]", " ".repeat(1_048_577));
        assert_eq!(execute(&[8], &input, 0, 0).0, 2);
    }
    #[test]
    fn maximum_builtin_payloads_are_not_limited_by_outer_json_rules() {
        let bytes = format!("[\"V\",\"hex:{}\"]", "ff".repeat(1_048_576));
        let (status, canonical, _) = execute(&[4], &bytes, 1, 0);
        assert_eq!(status, 0);
        assert_eq!(canonical, bytes);
        let text = format!("[\"V\",\"{}\"]", "\u{7f}".repeat(1_048_576));
        let (status, canonical, _) = execute(&[3], &text, 1, 0);
        assert_eq!(status, 0);
        assert_eq!(canonical.len(), 6_291_464);
        let list = format!("[\"V\",\"[{}]\"]", vec!["0"; 65_536].join(","));
        assert_eq!(execute(&[5], &list, 1, 0).0, 0);
        let json = format!("[\"V\",\"[{}]\"]", vec!["null"; 65_535].join(","));
        assert_eq!(execute(&[8], &json, 1, 0).0, 0);
    }
    #[test]
    fn canonicalizes_all_builtin_payload_transports() {
        let fixtures = [
            (0, "+0007", "7"),
            (1, "-0", "-0e0"),
            (1, "+nan", "nan"),
            (2, "true", "true"),
            (3, "a\\n\\u0085", "a\\n\\u0085"),
            (4, "hex:00ff", "hex:00ff"),
            (5, "[1, -0]", "[1,0]"),
            (6, "[+1, nan, -0]", "[1e0,nan,-0e0]"),
            (7, "[\\\"a\\\", \\\"b\\\"]", "[\\\"a\\\",\\\"b\\\"]"),
            (
                8,
                "{\\\"a\\\":1e02,\\\"a\\\":2}",
                "{\\\"a\\\":1e02,\\\"a\\\":2}",
            ),
        ];
        for (code, input, want) in fixtures {
            let (status, value, _) = execute(&[code], &format!("[\"V\",\"{input}\"]"), 1, 0);
            assert_eq!(status, 0, "{code} {input}");
            assert_eq!(value, format!("[\"V\",\"{want}\"]"));
        }
    }
    #[test]
    fn validates_unselected_leaf_and_preserves_failed_descriptor() {
        assert_eq!(execute(&[0], "[\"V\",\"bad\"]", 3, 0).0, 1);
        assert_eq!(execute(&[5], "[\"V\",\"[01]\"]", 3, 0).0, 1);
        assert_eq!(
            execute(&[1], "[\"V\",\"inf\"]", 4, 0).2.scalar,
            f64::INFINITY.to_bits()
        );
    }
    #[test]
    fn schema_rejects_bad_references_and_trailing_bytes() {
        let good = schema(&[0]);
        assert_eq!(unsafe { schema_check(good.as_ptr(), good.len() as u64) }, 0);
        let bad = schema(&[10]);
        assert_eq!(unsafe { schema_check(bad.as_ptr(), bad.len() as u64) }, 3);
        let mut extra = good;
        extra.push(0);
        assert_eq!(
            unsafe { schema_check(extra.as_ptr(), extra.len() as u64) },
            3
        );
    }
}
