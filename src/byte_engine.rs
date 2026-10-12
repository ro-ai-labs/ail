//! Allocation-free Bytes operations shared by the VM and embedded native runtime.
#[repr(C)]
pub struct ByteRequest {
    pub kind: u64,
    pub left: *const u8,
    pub left_len: u64,
    pub right: *const u8,
    pub right_len: u64,
    pub first: i64,
    pub second: i64,
    pub output: *mut u8,
    pub output_capacity: u64,
}
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct ByteResult {
    pub output_len: u64,
    pub scalar: i64,
}
const MAX_RAW: usize = 1_048_576;
const MAX_TRANSPORT: usize = 4 + 2 * MAX_RAW;
const HEX: &[u8; 16] = b"0123456789abcdef";

/// Validate canonical transport and return its raw byte length.
pub fn validate_transport(value: &[u8]) -> Result<usize, u64> {
    if value.len() > MAX_TRANSPORT {
        return Err(2);
    }
    if !value.starts_with(b"hex:")
        || !value.len().is_multiple_of(2)
        || !value[4..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err(1);
    }
    Ok((value.len() - 4) / 2)
}

struct Inputs<'a> {
    left: &'a [u8],
    right: &'a [u8],
    raw: usize,
    other_raw: usize,
}
unsafe fn input<'a>(
    pointer: *const u8,
    length: u64,
    text: bool,
    raw: bool,
) -> Result<&'a [u8], u64> {
    if pointer.is_null() {
        return Err(if text { 3 } else { 1 });
    }
    let bound = if text || raw { MAX_RAW } else { MAX_TRANSPORT };
    if length > bound as u64 {
        return Err(if text { 4 } else { 2 });
    }
    // The caller guarantees readable storage; the bounded conversion fits usize.
    let value = unsafe { core::slice::from_raw_parts(pointer, length as usize) };
    if text {
        core::str::from_utf8(value).map_err(|_| 3u64)?;
    }
    Ok(value)
}
unsafe fn inputs(request: &ByteRequest) -> Result<Inputs<'_>, u64> {
    if request.kind > 11 {
        return Err(6);
    }
    let text = matches!(request.kind, 5 | 7);
    let raw = request.kind == 10;
    let left = unsafe { input(request.left, request.left_len, text, raw)? };
    let size = if text || raw {
        left.len()
    } else {
        validate_transport(left)?
    };
    if request.kind == 7
        && (!left.len().is_multiple_of(2) || !left.iter().all(u8::is_ascii_hexdigit))
    {
        return Err(1);
    }
    let (right, other_raw) = if matches!(request.kind, 3 | 9) {
        let right = unsafe { input(request.right, request.right_len, false, false)? };
        (right, validate_transport(right)?)
    } else {
        (&[][..], 0)
    };
    Ok(Inputs {
        left,
        right,
        raw: size,
        other_raw,
    })
}

/// Check kind and input transports without reading output or numeric operands.
/// # Safety
/// A non-null request must be valid and aligned. Non-null input pointers must
/// address their declared readable lengths. Output is ignored.
pub unsafe extern "C" fn validate(request: *const ByteRequest) -> u64 {
    if request.is_null() {
        return 6;
    }
    match unsafe { inputs(&*request) } {
        Ok(_) => 0,
        Err(status) => status,
    }
}
fn nibble(byte: u8) -> u8 {
    if byte <= b'9' {
        byte - b'0'
    } else {
        byte - b'a' + 10
    }
}
fn byte_at(transport: &[u8], index: usize) -> u8 {
    (nibble(transport[4 + index * 2]) << 4) | nibble(transport[5 + index * 2])
}
fn index(value: i64, length: usize) -> Result<usize, u64> {
    let value = if value < 0 {
        (length as i64).saturating_add(value)
    } else {
        value
    };
    if value < 0 || value >= length as i64 {
        Err(5)
    } else {
        Ok(value as usize)
    }
}
fn bound(value: i64, length: usize) -> usize {
    let value = if value < 0 {
        (length as i64).saturating_add(value)
    } else {
        value
    };
    value.clamp(0, length as i64) as usize
}
// Validate decoded UTF-8 without writing output, so an invalid sequence cannot
// modify either the result descriptor or its caller-provided output buffer.
fn decoded_utf8(transport: &[u8], length: usize) -> bool {
    let mut position = 0;
    while position < length {
        let first = byte_at(transport, position);
        let (count, low, high) = match first {
            0..=0x7f => {
                position += 1;
                continue;
            }
            0xc2..=0xdf => (1, 0x80, 0xbf),
            0xe0 => (2, 0xa0, 0xbf),
            0xe1..=0xec | 0xee..=0xef => (2, 0x80, 0xbf),
            0xed => (2, 0x80, 0x9f),
            0xf0 => (3, 0x90, 0xbf),
            0xf1..=0xf3 => (3, 0x80, 0xbf),
            0xf4 => (3, 0x80, 0x8f),
            _ => return false,
        };
        if position + count >= length {
            return false;
        }
        let second = byte_at(transport, position + 1);
        if second < low || second > high {
            return false;
        }
        for offset in 2..=count {
            if !(0x80..=0xbf).contains(&byte_at(transport, position + offset)) {
                return false;
            }
        }
        position += count + 1;
    }
    true
}
unsafe fn execute(request: &ByteRequest, result: *mut ByteResult) -> Result<(), u64> {
    let values = unsafe { inputs(request)? };
    let mut scalar = 0;
    let mut start = 0;
    let mut end = values.raw;
    let output_len = match request.kind {
        0 => {
            scalar = values.raw as i64;
            0
        }
        1 => {
            scalar = byte_at(values.left, index(request.first, values.raw)?) as i64;
            0
        }
        2 => {
            start = bound(request.first, values.raw);
            end = bound(request.second, values.raw).max(start);
            4 + 2 * (end - start)
        }
        3 => {
            let length = values.raw + values.other_raw;
            if length > MAX_RAW {
                return Err(2);
            }
            4 + 2 * length
        }
        4 => {
            start = index(request.first, values.raw)?;
            if !(0..=255).contains(&request.second) {
                return Err(1);
            }
            values.left.len()
        }
        5 | 10 => 4 + 2 * values.raw,
        6 => {
            if !decoded_utf8(values.left, values.raw) {
                return Err(3);
            }
            values.raw
        }
        7 => 4 + values.left.len(),
        8 => {
            let length = 2 * values.raw;
            if length > MAX_RAW {
                return Err(4);
            }
            length
        }
        9 => {
            scalar = match values.left[4..].cmp(&values.right[4..]) {
                core::cmp::Ordering::Less => -1,
                core::cmp::Ordering::Equal => 0,
                core::cmp::Ordering::Greater => 1,
            };
            0
        }
        11 => values.raw,
        _ => unreachable!(),
    };
    if output_len as u64 > request.output_capacity || (output_len != 0 && request.output.is_null())
    {
        return Err(6);
    }
    let output = if output_len == 0 {
        &mut [][..]
    } else {
        // Only the exact bounded result region is accessed; the rest is untouched.
        unsafe { core::slice::from_raw_parts_mut(request.output, output_len) }
    };
    match request.kind {
        2 => {
            output[..4].copy_from_slice(b"hex:");
            output[4..].copy_from_slice(&values.left[4 + 2 * start..4 + 2 * end]);
        }
        3 => {
            output[..values.left.len()].copy_from_slice(values.left);
            output[values.left.len()..].copy_from_slice(&values.right[4..]);
        }
        4 => {
            output.copy_from_slice(values.left);
            output[4 + 2 * start] = HEX[(request.second >> 4) as usize];
            output[5 + 2 * start] = HEX[(request.second & 15) as usize];
        }
        5 | 10 => {
            output[..4].copy_from_slice(b"hex:");
            for (pair, byte) in output[4..].chunks_exact_mut(2).zip(values.left) {
                pair[0] = HEX[(byte >> 4) as usize];
                pair[1] = HEX[(byte & 15) as usize];
            }
        }
        6 | 11 => {
            for (i, byte) in output.iter_mut().enumerate() {
                *byte = byte_at(values.left, i);
            }
        }
        7 => {
            output[..4].copy_from_slice(b"hex:");
            for (out, byte) in output[4..].iter_mut().zip(values.left) {
                *out = byte.to_ascii_lowercase();
            }
        }
        8 => output.copy_from_slice(&values.left[4..]),
        _ => {}
    }
    unsafe {
        result.write(ByteResult {
            output_len: output_len as u64,
            scalar,
        })
    };
    Ok(())
}
/// Revalidate inputs, compute the operation, and publish its descriptor on success.
/// # Safety
/// Request/inputs must satisfy validate's contract. Output must be writable,
/// disjoint from inputs and result, and at least output_capacity bytes. A non-null
/// result must be valid and aligned. The request itself must not overlap output.
pub unsafe extern "C" fn run(request: *const ByteRequest, result: *mut ByteResult) -> u64 {
    if request.is_null() || result.is_null() {
        return 6;
    }
    match unsafe { execute(&*request, result) } {
        Ok(()) => 0,
        Err(status) => status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const MAX: usize = 1_048_576;
    fn request(kind: u64, left: &[u8], right: &[u8], first: i64, second: i64) -> ByteRequest {
        ByteRequest {
            kind,
            left: left.as_ptr(),
            left_len: left.len() as u64,
            right: right.as_ptr(),
            right_len: right.len() as u64,
            first,
            second,
            output: core::ptr::null_mut(),
            output_capacity: 0,
        }
    }
    fn perform(
        kind: u64,
        left: &[u8],
        right: &[u8],
        first: i64,
        second: i64,
    ) -> Result<(Vec<u8>, i64), u64> {
        let mut req = request(kind, left, right, first, second);
        let mut buffer = vec![0xa5; 4 + 2 * MAX];
        req.output = buffer.as_mut_ptr();
        req.output_capacity = buffer.len() as u64;
        let mut result = ByteResult {
            output_len: u64::MAX,
            scalar: i64::MIN,
        };
        let status = unsafe { run(&req, &mut result) };
        if status != 0 {
            assert_eq!(
                result,
                ByteResult {
                    output_len: u64::MAX,
                    scalar: i64::MIN
                }
            );
            return Err(status);
        }
        buffer.truncate(result.output_len as usize);
        Ok((buffer, result.scalar))
    }
    #[test]
    fn abi_and_canonical_transport_are_exact() {
        assert_eq!(core::mem::size_of::<ByteRequest>(), 72);
        assert_eq!(core::mem::size_of::<ByteResult>(), 16);
        assert_eq!(core::mem::offset_of!(ByteRequest, output), 56);
        assert_eq!(core::mem::offset_of!(ByteResult, scalar), 8);
        assert_eq!(validate_transport(b"hex:"), Ok(0));
        assert_eq!(validate_transport(b"hex:00ff80"), Ok(3));
        for bad in [
            b"".as_slice(),
            b"HEX:",
            b"hex:0",
            b"hex:FF",
            b"hex:gg",
            b"hex:00 ",
            b"hex:\xff\xff",
        ] {
            assert_eq!(validate_transport(bad), Err(1), "{bad:?}");
        }
        assert_eq!(validate_transport(&vec![b'0'; 4 + 2 * MAX + 1]), Err(2));
    }
    #[test]
    fn every_byte_roundtrips_and_compares_unsigned() {
        let raw: Vec<_> = (0..=255).collect();
        let (encoded, _) = perform(10, &raw, b"", 0, 0).unwrap();
        assert_eq!(encoded.len(), 516);
        assert!(encoded.starts_with(b"hex:000102"));
        assert_eq!(perform(11, &encoded, b"", 0, 0).unwrap().0, raw);
        assert_eq!(perform(0, &encoded, b"", 0, 0).unwrap().1, 256);
        assert_eq!(perform(1, &encoded, b"", -1, 0).unwrap().1, 255);
        for (left, right, want) in [
            (b"hex:80".as_slice(), b"hex:7f".as_slice(), 1),
            (b"hex:ff", b"hex:ff00", -1),
            (b"hex:", b"hex:", 0),
        ] {
            assert_eq!(perform(9, left, right, 0, 0).unwrap().1, want);
        }
    }
    #[test]
    fn slices_concat_set_and_numeric_order() {
        assert_eq!(
            perform(2, b"hex:0080ff", b"", -2, i64::MAX).unwrap().0,
            b"hex:80ff"
        );
        assert_eq!(
            perform(2, b"hex:0080ff", b"", i64::MIN, -1).unwrap().0,
            b"hex:0080"
        );
        assert_eq!(perform(2, b"hex:0080ff", b"", 2, 1).unwrap().0, b"hex:");
        assert_eq!(
            perform(3, b"hex:00", b"hex:80ff", 0, 0).unwrap().0,
            b"hex:0080ff"
        );
        assert_eq!(
            perform(4, b"hex:0080ff", b"", -1, 1).unwrap().0,
            b"hex:008001"
        );
        assert_eq!(perform(4, b"hex:", b"", 0, 256), Err(5));
        assert_eq!(perform(4, b"hex:00", b"", 0, 256), Err(1));
        assert_eq!(perform(4, b"hex:00", b"", 0, -1), Err(1));
        assert_eq!(
            unsafe { validate(&request(4, b"hex:", b"", i64::MAX, i64::MAX)) },
            0
        );
    }
    #[test]
    fn strict_utf8_and_hex_conversions() {
        let text = "A\0é🦀".as_bytes();
        let (bytes, _) = perform(5, text, b"", 0, 0).unwrap();
        assert_eq!(perform(6, &bytes, b"", 0, 0).unwrap().0, text);
        for raw in [
            b"\xc0\x80".as_slice(),
            b"\xed\xa0\x80",
            b"\xf4\x90\x80\x80",
            b"\xe2\x82",
            b"\x80",
        ] {
            let encoded = perform(10, raw, b"", 0, 0).unwrap().0;
            assert_eq!(perform(6, &encoded, b"", 0, 0), Err(3));
            assert_eq!(perform(5, raw, b"", 0, 0), Err(3));
        }
        assert_eq!(
            perform(7, b"00aAFF80", b"", 0, 0).unwrap().0,
            b"hex:00aaff80"
        );
        assert_eq!(
            perform(8, b"hex:00aaff80", b"", 0, 0).unwrap().0,
            b"00aaff80"
        );
        for bad in [b"0".as_slice(), b"hex:", b" ff", b"gg"] {
            assert_eq!(perform(7, bad, b"", 0, 0), Err(1));
        }
        assert_eq!(perform(7, b"\xff\xff", b"", 0, 0), Err(3));
    }
    #[test]
    fn exact_limits_missing_capacity_and_untouched_failures() {
        let raw = vec![b'a'; MAX];
        let encoded = perform(10, &raw, b"", 0, 0).unwrap().0;
        assert_eq!(encoded.len(), 2 * MAX + 4);
        assert_eq!(perform(6, &encoded, b"", 0, 0).unwrap().0, raw);
        assert_eq!(perform(8, &encoded, b"", 0, 0), Err(4));
        assert_eq!(perform(3, &encoded, b"hex:00", 0, 0), Err(2));
        assert_eq!(perform(10, &vec![0; MAX + 1], b"", 0, 0), Err(2));
        assert_eq!(perform(5, &vec![0; MAX + 1], b"", 0, 0), Err(4));
        let half = perform(10, &vec![0; MAX / 2], b"", 0, 0).unwrap().0;
        assert_eq!(perform(8, &half, b"", 0, 0).unwrap().0.len(), MAX);
        let mut req = request(2, b"hex:00", b"", 0, 1);
        let mut result = ByteResult {
            output_len: 123,
            scalar: 456,
        };
        assert_eq!(unsafe { run(&req, &mut result) }, 6);
        assert_eq!(
            result,
            ByteResult {
                output_len: 123,
                scalar: 456
            }
        );
        req.kind = 0;
        assert_eq!(unsafe { run(&req, &mut result) }, 0);
        assert_eq!(
            result,
            ByteResult {
                output_len: 0,
                scalar: 1
            }
        );
        req.left = core::ptr::null();
        assert_eq!(unsafe { validate(&req) }, 1);
        req.kind = 5;
        assert_eq!(unsafe { validate(&req) }, 3);
        req.kind = 99;
        assert_eq!(unsafe { validate(&req) }, 6);
        assert_eq!(unsafe { validate(core::ptr::null()) }, 6);
        let bad = perform(10, b"\xff", b"", 0, 0).unwrap().0;
        let mut req = request(6, &bad, b"", 0, 0);
        let mut output = [0xa5; 8];
        req.output = output.as_mut_ptr();
        req.output_capacity = 8;
        assert_eq!(unsafe { run(&req, &mut result) }, 3);
        assert_eq!(output, [0xa5; 8]);
    }
}
