//! Allocation-free UTF-8/JSON scanning and canonical output sinks.
pub type Result<T> = core::result::Result<T, u64>;
pub trait Sink {
    fn put(&mut self, value: &[u8]) -> Result<()>;
}
pub struct Count(pub usize);
impl Sink for Count {
    fn put(&mut self, v: &[u8]) -> Result<()> {
        self.0 = self.0.checked_add(v.len()).ok_or(2u64)?;
        Ok(())
    }
}
pub struct Output<'a> {
    pub bytes: &'a mut [u8],
    pub len: usize,
}
impl Sink for Output<'_> {
    fn put(&mut self, v: &[u8]) -> Result<()> {
        let end = self.len.checked_add(v.len()).ok_or(3u64)?;
        self.bytes
            .get_mut(self.len..end)
            .ok_or(3u64)?
            .copy_from_slice(v);
        self.len = end;
        Ok(())
    }
}
pub struct Quoted<'a>(pub &'a mut dyn Sink);
impl Sink for Quoted<'_> {
    fn put(&mut self, bytes: &[u8]) -> Result<()> {
        for c in core::str::from_utf8(bytes).map_err(|_| 1u64)?.chars() {
            quote_char(c, self.0)?;
        }
        Ok(())
    }
}
pub fn quote_char(c: char, out: &mut dyn Sink) -> Result<()> {
    match c {
        '"' => out.put(b"\\\""),
        '\\' => out.put(b"\\\\"),
        '\n' => out.put(b"\\n"),
        '\r' => out.put(b"\\r"),
        '\t' => out.put(b"\\t"),
        c if c.is_control() => {
            let n = c as usize;
            let hex = b"0123456789abcdef";
            out.put(&[
                b'\\',
                b'u',
                hex[(n >> 12) & 15],
                hex[(n >> 8) & 15],
                hex[(n >> 4) & 15],
                hex[n & 15],
            ])
        }
        c => {
            let mut bytes = [0; 4];
            out.put(c.encode_utf8(&mut bytes).as_bytes())
        }
    }
}
pub fn quote(bytes: &[u8], out: &mut dyn Sink) -> Result<()> {
    out.put(b"\"")?;
    Quoted(out).put(bytes)?;
    out.put(b"\"")
}
pub struct Cursor<'a> {
    pub bytes: &'a [u8],
    pub at: usize,
}
impl<'a> Cursor<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self> {
        core::str::from_utf8(bytes).map_err(|_| 1u64)?;
        Ok(Self { bytes, at: 0 })
    }
    pub fn space(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.at += 1;
        }
    }
    pub fn take(&mut self, b: u8) -> bool {
        if self.bytes.get(self.at) == Some(&b) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    pub fn need(&mut self, b: u8) -> Result<()> {
        if self.take(b) { Ok(()) } else { Err(1) }
    }
    pub fn done(&mut self) -> Result<()> {
        self.space();
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(1)
        }
    }
    pub fn word(&mut self, s: &[u8]) -> Result<()> {
        if self.bytes[self.at..].starts_with(s) {
            self.at += s.len();
            Ok(())
        } else {
            Err(1)
        }
    }
    fn hex4(&mut self) -> Result<u32> {
        let mut n = 0;
        for _ in 0..4 {
            let b = *self.bytes.get(self.at).ok_or(1u64)?;
            self.at += 1;
            n = n * 16
                + match b {
                    b'0'..=b'9' => (b - b'0') as u32,
                    b'a'..=b'f' => (b - b'a' + 10) as u32,
                    b'A'..=b'F' => (b - b'A' + 10) as u32,
                    _ => return Err(1),
                };
        }
        Ok(n)
    }
    pub fn string(&mut self, out: &mut dyn Sink) -> Result<()> {
        self.need(b'"')?;
        loop {
            let b = *self.bytes.get(self.at).ok_or(1u64)?;
            if b == b'"' {
                self.at += 1;
                return Ok(());
            }
            if b == b'\\' {
                self.at += 1;
                let b = *self.bytes.get(self.at).ok_or(1u64)?;
                self.at += 1;
                let c = match b {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'/' => '/',
                    b'b' => '\u{8}',
                    b'f' => '\u{c}',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'u' => {
                        let first = self.hex4()?;
                        let n = if (0xd800..=0xdbff).contains(&first) {
                            self.need(b'\\')?;
                            self.need(b'u')?;
                            let second = self.hex4()?;
                            if !(0xdc00..=0xdfff).contains(&second) {
                                return Err(1);
                            }
                            0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00)
                        } else {
                            first
                        };
                        char::from_u32(n).ok_or(1u64)?
                    }
                    _ => return Err(1),
                };
                let mut buffer = [0; 4];
                out.put(c.encode_utf8(&mut buffer).as_bytes())?;
            } else {
                if b < 32 {
                    return Err(1);
                }
                let start = self.at;
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| *b >= 32 && *b != b'"' && *b != b'\\')
                {
                    self.at += 1;
                }
                out.put(&self.bytes[start..self.at])?;
            }
        }
    }
    pub fn quoted(&mut self, out: &mut dyn Sink) -> Result<()> {
        out.put(b"\"")?;
        self.string(&mut Quoted(out))?;
        out.put(b"\"")
    }
    pub fn number(&mut self) -> Result<&'a [u8]> {
        let start = self.at;
        self.take(b'-');
        if !self.take(b'0') {
            if !self
                .bytes
                .get(self.at)
                .is_some_and(|b| matches!(b, b'1'..=b'9'))
            {
                return Err(1);
            }
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        if self.take(b'.') {
            let p = self.at;
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
            if p == self.at {
                return Err(1);
            }
        }
        if self.take(b'e') || self.take(b'E') {
            let _ = self.take(b'+') || self.take(b'-');
            let p = self.at;
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
            if p == self.at {
                return Err(1);
            }
        }
        Ok(&self.bytes[start..self.at])
    }
}
pub fn json_value(
    c: &mut Cursor<'_>,
    out: &mut dyn Sink,
    depth: usize,
    count: &mut usize,
) -> Result<()> {
    c.space();
    *count += 1;
    if *count > 65_536 {
        return Err(2);
    }
    match c.bytes.get(c.at) {
        Some(b'"') => c.quoted(out),
        Some(b'n') => {
            c.word(b"null")?;
            out.put(b"null")
        }
        Some(b't') => {
            c.word(b"true")?;
            out.put(b"true")
        }
        Some(b'f') => {
            c.word(b"false")?;
            out.put(b"false")
        }
        Some(b'[' | b'{') => {
            if depth >= 64 {
                return Err(2);
            }
            let open = c.bytes[c.at];
            let close = if open == b'[' { b']' } else { b'}' };
            c.at += 1;
            out.put(&[open])?;
            c.space();
            if !c.take(close) {
                loop {
                    if open == b'{' {
                        c.space();
                        c.quoted(out)?;
                        c.space();
                        c.need(b':')?;
                        out.put(b":")?;
                    }
                    json_value(c, out, depth + 1, count)?;
                    c.space();
                    if c.take(close) {
                        break;
                    }
                    c.need(b',')?;
                    out.put(b",")?;
                }
            }
            out.put(&[close])
        }
        Some(b'-' | b'0'..=b'9') => out.put(c.number()?),
        _ => Err(1),
    }
}
