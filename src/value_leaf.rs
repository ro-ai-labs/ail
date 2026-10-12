//! Builtin transport compatibility for the shared variant engine.
use crate::value_transport::{self as t, Count, Cursor, Result, Sink};
const LIMIT: usize = 1_048_576;
fn integer(bytes: &[u8]) -> Result<i64> {
    let text = core::str::from_utf8(bytes).map_err(|_| 1u64)?;
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(1);
    }
    text.parse().map_err(|_| 1u64)
}
fn integer_write(n: i64, out: &mut dyn Sink) -> Result<()> {
    let mut buffer = [0; 20];
    let mut at = 20;
    let mut value = n.unsigned_abs();
    loop {
        at -= 1;
        buffer[at] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    if n < 0 {
        at -= 1;
        buffer[at] = b'-';
    }
    out.put(&buffer[at..])
}
fn floating(bytes: &[u8], out: &mut dyn Sink) -> Result<u64> {
    let bits = crate::float_number::parse(core::str::from_utf8(bytes).map_err(|_| 1u64)?).map_err(
        |e| {
            if e == crate::float_number::Error::Limit {
                2u64
            } else {
                1
            }
        },
    )?;
    let mut buffer = [0; 32];
    let len = crate::float_number::format(bits, &mut buffer).map_err(|_| 3u64)?;
    out.put(&buffer[..len])?;
    Ok(bits)
}
pub fn limit(kind: u32) -> usize {
    if kind == 4 { 2_097_156 } else { LIMIT }
}
pub fn write(kind: u32, bytes: &[u8], out: &mut dyn Sink) -> Result<u64> {
    if kind != 0 && bytes.len() > limit(kind) {
        return Err(2);
    }
    core::str::from_utf8(bytes).map_err(|_| 1u64)?;
    match kind {
        0 => {
            let n = integer(bytes)?;
            integer_write(n, out)?;
            Ok(n as u64)
        }
        1 => floating(bytes, out),
        2 => {
            let v = match bytes {
                b"true" => 1,
                b"false" => 0,
                _ => return Err(1),
            };
            out.put(bytes)?;
            Ok(v)
        }
        3 => {
            out.put(bytes)?;
            Ok(0)
        }
        4 => {
            crate::byte_engine::validate_transport(bytes)?;
            out.put(bytes)?;
            Ok(0)
        }
        5..=7 => {
            let mut c = Cursor::new(bytes)?;
            c.space();
            c.need(b'[')?;
            out.put(b"[")?;
            c.space();
            let mut count = 0;
            if !c.take(b']') {
                loop {
                    count += 1;
                    if count > 65_536 {
                        return Err(2);
                    }
                    c.space();
                    match kind {
                        5 => {
                            let token = c.number()?;
                            if token.iter().any(|b| matches!(b, b'.' | b'e' | b'E')) {
                                return Err(1);
                            }
                            integer_write(integer(token)?, out)?;
                        }
                        6 => {
                            let start = c.at;
                            while c.bytes.get(c.at).is_some_and(|b| {
                                !matches!(b, b',' | b']' | b' ' | b'\t' | b'\r' | b'\n')
                            }) {
                                c.at += 1;
                            }
                            floating(&c.bytes[start..c.at], out)?;
                        }
                        _ => c.quoted(out)?,
                    }
                    c.space();
                    if c.take(b']') {
                        break;
                    }
                    c.need(b',')?;
                    out.put(b",")?;
                }
            }
            c.done()?;
            out.put(b"]")?;
            Ok(0)
        }
        8 => {
            let mut c = Cursor::new(bytes)?;
            t::json_value(&mut c, out, 0, &mut 0)?;
            c.done()?;
            Ok(0)
        }
        _ => Err(3),
    }
}
pub fn measure(kind: u32, bytes: &[u8], owned: bool) -> Result<(usize, u64)> {
    let mut n = Count(0);
    let scalar = write(kind, bytes, &mut n)?;
    if owned && kind != 0 && n.0 > limit(kind) {
        return Err(2);
    }
    Ok((n.0, scalar))
}
