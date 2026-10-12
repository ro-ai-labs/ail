//! Allocation-free binary64 semantics shared by the VM and native runtime.
use core::fmt;

pub const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;
pub const INPUT_LIMIT: usize = 1_048_576;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Error {
    Invalid = 1,
    Limit = 2,
    DivisionByZero = 3,
    Range = 4,
    Buffer = 5,
}
impl Error {
    pub const fn message(self) -> &'static str {
        match self {
            Self::Invalid => "AIL-RUNTIME-FLOAT: invalid floating value",
            Self::Limit => "AIL-RUNTIME-FLOAT-LIMIT: floating input limit exceeded",
            Self::DivisionByZero => "AIL-RUNTIME-DIVZERO: floating divisor is zero",
            Self::Range => "AIL-RUNTIME-FLOAT: floating value is outside the integer range",
            Self::Buffer => "AIL-RUNTIME-FLOAT: floating output buffer is too small",
        }
    }
    pub const fn status(self) -> usize {
        self as usize
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}
pub fn normalize(bits: u64) -> u64 {
    if bits & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000 && bits & 0x000f_ffff_ffff_ffff != 0 {
        CANONICAL_NAN
    } else {
        bits
    }
}

/// Parse only the specified ASCII transport grammar. Core's decimal conversion
/// supplies correctly rounded binary64; grammar validation excludes its aliases.
pub fn parse(text: &str) -> Result<u64, Error> {
    if text.len() > INPUT_LIMIT {
        return Err(Error::Limit);
    }
    match text {
        "inf" | "+inf" => return Ok(f64::INFINITY.to_bits()),
        "-inf" => return Ok(f64::NEG_INFINITY.to_bits()),
        "nan" | "+nan" | "-nan" => return Ok(CANONICAL_NAN),
        _ => {}
    }
    let bytes = text.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    let integer_digits = index - start;
    let mut fractional_digits = 0;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        fractional_digits = index - start;
    }
    if integer_digits + fractional_digits == 0 {
        return Err(Error::Invalid);
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == start {
            return Err(Error::Invalid);
        }
    }
    if index != bytes.len() {
        return Err(Error::Invalid);
    }
    text.parse::<f64>()
        .map(|value| normalize(value.to_bits()))
        .map_err(|_| Error::Invalid)
}

struct Output<'a> {
    bytes: &'a mut [u8],
    length: usize,
}

/// Parse one strict JSON number token with a finite binary64 result.
pub fn parse_json(text: &str) -> Result<u64, Error> {
    if text.len() > INPUT_LIMIT {
        return Err(Error::Limit);
    }
    let bytes = text.as_bytes();
    let mut position = usize::from(bytes.first() == Some(&b'-'));
    match bytes.get(position) {
        Some(b'0') => position += 1,
        Some(b'1'..=b'9') => {
            position += 1;
            while bytes.get(position).is_some_and(u8::is_ascii_digit) {
                position += 1;
            }
        }
        _ => return Err(Error::Invalid),
    }
    if bytes.get(position) == Some(&b'.') {
        position += 1;
        let start = position;
        while bytes.get(position).is_some_and(u8::is_ascii_digit) {
            position += 1;
        }
        if start == position {
            return Err(Error::Invalid);
        }
    }
    if matches!(bytes.get(position), Some(b'e' | b'E')) {
        position += 1;
        if matches!(bytes.get(position), Some(b'+' | b'-')) {
            position += 1;
        }
        let start = position;
        while bytes.get(position).is_some_and(u8::is_ascii_digit) {
            position += 1;
        }
        if start == position {
            return Err(Error::Invalid);
        }
    }
    if position != bytes.len() {
        return Err(Error::Invalid);
    }
    let bits = parse(text)?;
    if f64::from_bits(bits).is_finite() {
        Ok(bits)
    } else {
        Err(Error::Invalid)
    }
}
impl fmt::Write for Output<'_> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let remaining = self.bytes.get_mut(self.length..).ok_or(fmt::Error)?;
        let destination = remaining.get_mut(..value.len()).ok_or(fmt::Error)?;
        destination.copy_from_slice(value.as_bytes());
        self.length += value.len();
        Ok(())
    }
}

/// Shortest round-trip lowercase scientific notation, always at most 32 bytes.
pub fn format(bits: u64, output: &mut [u8]) -> Result<usize, Error> {
    use core::fmt::Write;
    let value = f64::from_bits(bits);
    let mut output = Output {
        bytes: output,
        length: 0,
    };
    let result = if value.is_nan() {
        output.write_str("nan")
    } else if value == f64::INFINITY {
        output.write_str("inf")
    } else if value == f64::NEG_INFINITY {
        output.write_str("-inf")
    } else {
        write!(&mut output, "{value:e}")
    };
    result.map_err(|_| Error::Buffer)?;
    Ok(output.length)
}

/// Add0/Sub1/Mul2/Div3/Rem4. NaNs have one canonical transport representation.
pub fn binary(op: u8, left: u64, right: u64) -> Result<u64, Error> {
    let left = f64::from_bits(left);
    let right = f64::from_bits(right);
    if matches!(op, 3 | 4) && right == 0.0 {
        return Err(Error::DivisionByZero);
    }
    let result = match op {
        0 => left + right,
        1 => left - right,
        2 => left * right,
        3 => left / right,
        4 => {
            let remainder = fmod(left, right);
            if remainder == 0.0 {
                f64::from_bits(right.to_bits() & (1 << 63))
            } else if (remainder < 0.0) != (right < 0.0) {
                remainder + right
            } else {
                remainder
            }
        }
        _ => return Err(Error::Invalid),
    };
    Ok(normalize(result.to_bits()))
}

/// Eq0/Ne1/Lt2/Le3/Gt4/Ge5; unordered IEEE comparisons remain unordered.
pub fn compare(op: u8, left: u64, right: u64) -> bool {
    let left = f64::from_bits(left);
    let right = f64::from_bits(right);
    match op {
        0 => left == right,
        1 => left != right,
        2 => left < right,
        3 => left <= right,
        4 => left > right,
        5 => left >= right,
        _ => false,
    }
}
pub fn from_integer(value: i64) -> u64 {
    (value as f64).to_bits()
}
pub fn to_integer(bits: u64) -> Result<i64, Error> {
    let value = f64::from_bits(bits);
    // MAX rounds to 2^63 as binary64; compare to that exclusive upper bound.
    if !value.is_finite() || !(-9223372036854775808.0..9223372036854775808.0).contains(&value) {
        Err(Error::Range)
    } else {
        Ok(value as i64)
    }
}

/// Exact fmod using integer significands. This avoids libc/libm and avoids
/// quotient rounding when the exponent gap is large. Each subtraction is exact.
fn fmod(left: f64, right: f64) -> f64 {
    let sign = left.to_bits() & (1 << 63);
    let x = left.to_bits() & !(1 << 63);
    let y = right.to_bits() & !(1 << 63);
    if x >= f64::INFINITY.to_bits() || y > f64::INFINITY.to_bits() || y == 0 {
        return f64::from_bits(CANONICAL_NAN);
    }
    if x < y {
        return left;
    }
    if x == y {
        return f64::from_bits(sign);
    }
    fn parts(bits: u64) -> (u64, i32) {
        let exponent = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & 0x000f_ffff_ffff_ffff;
        if exponent != 0 {
            ((1 << 52) | fraction, exponent - 1023)
        } else {
            let shift = fraction.leading_zeros() - 11;
            (fraction << shift, -1022 - shift as i32)
        }
    }
    let (mut remainder, mut exponent) = parts(x);
    let (divisor, divisor_exponent) = parts(y);
    while exponent > divisor_exponent {
        if remainder >= divisor {
            remainder -= divisor;
        }
        if remainder == 0 {
            return f64::from_bits(sign);
        }
        remainder <<= 1;
        exponent -= 1;
    }
    if remainder >= divisor {
        remainder -= divisor;
    }
    if remainder == 0 {
        return f64::from_bits(sign);
    }
    let shift = remainder.leading_zeros() - 11;
    remainder <<= shift;
    exponent -= shift as i32;
    let magnitude = if exponent >= -1022 {
        ((exponent + 1023) as u64) << 52 | (remainder & 0x000f_ffff_ffff_ffff)
    } else {
        remainder >> (-1022 - exponent)
    };
    f64::from_bits(sign | magnitude)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bits(value: f64) -> u64 {
        value.to_bits()
    }
    fn rendered(bits: u64) -> String {
        let mut output = [0u8; 32];
        let length = format(bits, &mut output).unwrap();
        String::from_utf8(output[..length].to_vec()).unwrap()
    }
    #[test]
    fn decimal_parsing_rounds_ties_and_extreme_exponents_to_binary64() {
        for (text, expected) in [
            (
                "1.00000000000000011102230246251565404236316680908203125",
                0x3ff0_0000_0000_0000,
            ),
            (
                "1.00000000000000033306690738754696212708950042724609375",
                0x3ff0_0000_0000_0002,
            ),
            ("9007199254740993", 0x4340_0000_0000_0000),
            ("4.9406564584124654e-324", 1),
            ("2.4703282292062327e-324", 0),
            ("2.4703282292062328e-324", 1),
            ("2.2250738585072014e-308", 0x0010_0000_0000_0000),
            ("1.7976931348623157e308", 0x7fef_ffff_ffff_ffff),
            ("1.7976931348623159e308", 0x7ff0_0000_0000_0000),
            ("-1e-999999999999999999999999", 0x8000_0000_0000_0000),
            ("-0e999999999999999999999999", 0x8000_0000_0000_0000),
        ] {
            assert_eq!(parse(text), Ok(expected), "{text}");
        }
    }
    #[test]
    fn transport_accepts_only_the_declared_ascii_grammar_and_bounded_input() {
        for (text, expected) in [
            (".5", bits(0.5)),
            ("1.", bits(1.0)),
            ("+001.250E+2", bits(125.0)),
            ("inf", bits(f64::INFINITY)),
            ("+inf", bits(f64::INFINITY)),
            ("-inf", bits(f64::NEG_INFINITY)),
            ("nan", CANONICAL_NAN),
            ("+nan", CANONICAL_NAN),
            ("-nan", CANONICAL_NAN),
        ] {
            assert_eq!(parse(text), Ok(expected), "{text}");
        }
        for text in [
            "", "+", "-", ".", "1e", "1e+", "1e-", "1e1.0", "1.2.3", "1_0", " 1", "1 ", "0x1p2",
            "NaN", "INF", "Infinity", "nan(1)", "1\0", "１", "1\n", "+-1", "--0",
        ] {
            assert_eq!(parse(text), Err(Error::Invalid), "{text:?}");
        }
        let limit = "0".repeat(INPUT_LIMIT);
        assert_eq!(parse(&limit), Ok(0));
        assert_eq!(parse(&(limit + "0")), Err(Error::Limit));
    }
    #[test]
    fn json_numbers_require_strict_grammar_and_finite_rounded_results() {
        for text in [
            "0",
            "-0",
            "1.25",
            "1e2",
            "1E+2",
            "-1E-9999",
            "5e-324",
            "1.7976931348623157e308",
        ] {
            assert_eq!(parse_json(text), parse(text), "{text}");
        }
        for text in [
            "+1", "01", "-01", ".5", "1.", "1.e2", "nan", "+inf", "-inf", "1e9999", "-1e9999",
            "1e", "1e+", "1e-", "1_0", " 1", "1 ", "", "-", "true", "[1]", "１", "1\0",
        ] {
            assert_eq!(parse_json(text), Err(Error::Invalid), "{text:?}");
        }
        let exact = format!("0.{}", "0".repeat(INPUT_LIMIT - 2));
        assert_eq!(parse_json(&exact), Ok(0));
        assert_eq!(parse_json(&(exact + "0")), Err(Error::Limit));
    }
    #[test]
    fn canonical_format_is_shortest_scientific_preserves_zero_and_normalizes_nan() {
        for (value, expected) in [
            (bits(1.5), "1.5e0"),
            (0, "0e0"),
            (1 << 63, "-0e0"),
            (1, "5e-324"),
            (bits(f64::MAX), "1.7976931348623157e308"),
            (bits(f64::INFINITY), "inf"),
            (bits(f64::NEG_INFINITY), "-inf"),
            (0xfff0_0000_0000_0001, "nan"),
        ] {
            assert_eq!(rendered(value), expected);
            assert_eq!(parse(expected), Ok(normalize(value)));
        }
        assert_eq!(format(bits(1.5), &mut [0; 2]), Err(Error::Buffer));
    }
    #[test]
    fn arithmetic_obeys_ieee_values_and_divisor_sign_remainder() {
        for (left, right, expected) in [
            (-7.0, 3.0, 2.0),
            (7.0, -3.0, -2.0),
            (-7.0, -3.0, -1.0),
            (-6.0, 3.0, 0.0),
            (6.0, -3.0, -0.0),
            (0.0, -3.0, -0.0),
            (1.0, f64::NEG_INFINITY, f64::NEG_INFINITY),
            (-1.0, f64::INFINITY, f64::INFINITY),
            (1.0, f64::INFINITY, 1.0),
            (-1.0, f64::NEG_INFINITY, -1.0),
        ] {
            assert_eq!(
                binary(4, bits(left), bits(right)),
                Ok(bits(expected)),
                "{left}%{right}"
            );
        }
        assert_eq!(
            binary(0, bits(f64::MAX), bits(f64::MAX)),
            Ok(bits(f64::INFINITY))
        );
        assert_eq!(
            binary(1, bits(f64::INFINITY), bits(f64::INFINITY)),
            Ok(CANONICAL_NAN)
        );
        assert_eq!(binary(2, bits(-0.0), bits(2.0)), Ok(bits(-0.0)));
        assert_eq!(binary(3, bits(1.0), bits(f64::INFINITY)), Ok(bits(0.0)));
        assert_eq!(binary(4, bits(f64::INFINITY), bits(1.0)), Ok(CANONICAL_NAN));
        for operation in [3, 4] {
            for zero in [0.0, -0.0] {
                assert_eq!(
                    binary(operation, CANONICAL_NAN, bits(zero)),
                    Err(Error::DivisionByZero)
                );
            }
        }
    }
    #[test]
    fn unordered_comparisons_and_integer_conversions_have_explicit_bounds() {
        for operation in 0..6 {
            assert_eq!(
                compare(operation, CANONICAL_NAN, CANONICAL_NAN),
                operation == 1
            );
        }
        assert!(compare(0, bits(-0.0), bits(0.0)));
        assert!(compare(2, bits(f64::NEG_INFINITY), bits(0.0)));
        assert!(compare(3, bits(2.0), bits(2.0)));
        assert!(compare(4, bits(f64::INFINITY), bits(f64::MAX)));
        assert!(compare(5, bits(2.0), bits(2.0)));
        assert_eq!(from_integer(9007199254740993), bits(9007199254740992.0));
        assert_eq!(from_integer(i64::MAX), bits(9223372036854775808.0));
        for (value, expected) in [
            (-1.75, -1),
            (1.75, 1),
            (-0.0, 0),
            (-9223372036854775808.0, i64::MIN),
            (9223372036854774784.0, 9223372036854774784),
        ] {
            assert_eq!(to_integer(bits(value)), Ok(expected));
        }
        for value in [
            9223372036854775808.0,
            -9223372036854777856.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            assert_eq!(to_integer(bits(value)), Err(Error::Range));
        }
    }
    #[test]
    fn seeded_remainders_match_independent_host_fmod_for_wide_exponent_gaps() {
        let mut state = 0xded0_5eed_1095_8643u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..20_000 {
            let left = f64::from_bits(next());
            let right = f64::from_bits(next());
            if right == 0.0 {
                continue;
            }
            let remainder = left % right;
            let expected = if remainder == 0.0 {
                f64::from_bits(right.to_bits() & (1 << 63))
            } else if (remainder < 0.0) != (right < 0.0) {
                remainder + right
            } else {
                remainder
            };
            assert_eq!(
                binary(4, left.to_bits(), right.to_bits()),
                Ok(normalize(expected.to_bits())),
                "left={:016x}, right={:016x}",
                left.to_bits(),
                right.to_bits()
            );
        }
    }

    #[test]
    fn seeded_canonical_formats_fit_numeric_slots_and_preserve_every_bit() {
        let mut state = 0xb17f_64de_c1a1_0001u64;
        for _ in 0..20_000 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let mut output = [0u8; 32];
            let length = format(state, &mut output).unwrap();
            let text = core::str::from_utf8(&output[..length]).unwrap();
            assert_eq!(
                parse(text),
                Ok(normalize(state)),
                "bits={state:016x} text={text}"
            );
        }
    }
}
