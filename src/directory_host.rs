//! Allocator-free Linux x86-64 directory effects shared by VM and native callers.

#[repr(C)]
pub struct DirectoryRequest {
    pub kind: u64,
    pub root: *const u8,
    pub path: *const u8,
    pub path_len: u64,
    pub source: *const u8,
    pub source_len: u64,
    pub output: *mut u8,
    pub output_capacity: u64,
    pub scratch: *mut u8,
    pub scratch_capacity: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct DirectoryResult {
    pub output_len: u64,
    pub entry_count: u64,
}

/// # Safety
/// Request is live; present path/source pointers are readable for their lengths.
/// Root and output/scratch are not accessed by this validation entry point.
pub unsafe extern "C" fn ail_directory_validate(request: *const DirectoryRequest) -> u64 {
    if request.is_null() {
        return IO;
    }
    unsafe { validate(&*request) }.err().unwrap_or(0)
}

/// # Safety
/// Request/result are live and disjoint. Root is a caller-validated absolute C
/// string, at most 4095 bytes. Inputs remain immutable through the call. Output
/// and 8-byte-aligned scratch have the exact documented capacities and are
/// disjoint from each other, inputs, request and result. Input slices may alias.
pub unsafe extern "C" fn ail_directory_run(
    request: *const DirectoryRequest,
    result: *mut DirectoryResult,
) -> u64 {
    if request.is_null() || result.is_null() {
        return IO;
    }
    let request = unsafe { &*request };
    if let Err(status) = unsafe { validate(request) } {
        return status;
    }
    if request.root.is_null()
        || request.output.is_null()
        || request.scratch.is_null()
        || request.output_capacity != CAPACITY as u64
        || request.scratch_capacity != SCRATCH as u64
        || !(request.scratch as usize).is_multiple_of(8)
    {
        return IO;
    }
    let mut engine = Engine { fds: [-1; 2] };
    let outcome = unsafe { engine.execute(request) };
    engine.close();
    match outcome {
        Ok(value) => {
            unsafe {
                result.write(value);
            }
            0
        }
        Err(status) => status,
    }
}

const PATH: u64 = 1;
const TEXT: u64 = 2;
const TEXT_LIMIT: u64 = 3;
const LIST_LIMIT: u64 = 4;
const IO: u64 = 5;
const LIMIT: usize = 1_048_576;
const CAPACITY: usize = LIMIT + 1;
const SCRATCH: usize = 0x311000;
const COUNT: usize = 65_536;

unsafe fn text<'a>(data: *const u8, len: u64) -> Result<&'a [u8], u64> {
    if data.is_null() {
        return Err(TEXT);
    }
    if len > LIMIT as u64 {
        return Err(TEXT_LIMIT);
    }
    let bytes = unsafe { core::slice::from_raw_parts(data, len as usize) };
    core::str::from_utf8(bytes).map_err(|_| TEXT)?;
    Ok(bytes)
}
unsafe fn validate(request: &DirectoryRequest) -> Result<(), u64> {
    if request.kind > 2 {
        return Err(IO);
    }
    let path = unsafe { text(request.path, request.path_len) }?;
    if path.len() > 4095
        || path.contains(&0)
        || (path.is_empty() && request.kind != 0)
        || (!path.is_empty()
            && path
                .split(|b| *b == b'/')
                .any(|p| p.is_empty() || p == b"." || p == b".."))
    {
        return Err(PATH);
    }
    if request.kind == 2 {
        unsafe { text(request.source, request.source_len) }?;
    }
    Ok(())
}

#[inline(always)]
fn syscall(number: usize, a: usize, b: usize, c: usize, d: usize, e: usize, f: usize) -> isize {
    let result: isize;
    unsafe {
        core::arch::asm!("syscall", inlateout("rax") number => result,
            in("rdi") a, in("rsi") b, in("rdx") c, in("r10") d, in("r8") e, in("r9") f,
            lateout("rcx") _, lateout("r11") _, options(nostack));
    }
    result
}
fn call(number: usize, a: usize, b: usize, c: usize, d: usize) -> Result<usize, u64> {
    loop {
        let result = syscall(number, a, b, c, d, 0, 0);
        if result == -4 {
            continue;
        }
        return if result >= 0 {
            Ok(result as usize)
        } else {
            Err(match -result {
                1 | 2 | 6 | 13 | 19 | 20 | 21 | 18 | 30 | 36 | 40 => PATH,
                _ => IO,
            })
        };
    }
}
struct Engine {
    fds: [isize; 2],
}
impl Engine {
    fn close(&mut self) {
        for fd in &mut self.fds {
            let owned = core::mem::replace(fd, -1);
            if owned >= 0 {
                syscall(3, owned as usize, 0, 0, 0, 0, 0);
            }
        }
    }
    unsafe fn execute(&mut self, request: &DirectoryRequest) -> Result<DirectoryResult, u64> {
        let scratch = unsafe { core::slice::from_raw_parts_mut(request.scratch, SCRATCH) };
        let path = &mut scratch[0x310000..];
        let len = request.path_len as usize;
        if len == 0 {
            path[0] = b'.';
            path[1] = 0;
        } else {
            path[..len].copy_from_slice(unsafe { core::slice::from_raw_parts(request.path, len) });
            path[len] = 0;
        }
        self.fds[0] = call(
            257,
            (-100_isize) as usize,
            request.root as usize,
            0x290000,
            0,
        )? as isize;
        let flags = match request.kind {
            0 => 0x90000_u64,
            1 => 0x80900,
            _ => 0x80941,
        };
        let how = [flags, if request.kind == 2 { 0o666 } else { 0 }, 0x0c];
        self.fds[1] = call(
            437,
            self.fds[0] as usize,
            path.as_ptr() as usize,
            how.as_ptr() as usize,
            24,
        )? as isize;
        let fd = self.fds[1] as usize;
        let output = unsafe { core::slice::from_raw_parts_mut(request.output, CAPACITY) };
        if request.kind == 0 {
            return listing(fd, scratch, output);
        }
        let mut stat = [0_u64; 18];
        call(5, fd, stat.as_mut_ptr() as usize, 0, 0)?;
        if stat[3] & 0xf000 != 0x8000 {
            return Err(PATH);
        }
        if request.kind == 1 {
            let mut used = 0;
            loop {
                let read = call(
                    0,
                    fd,
                    output[used..].as_mut_ptr() as usize,
                    CAPACITY - used,
                    0,
                )?;
                if read > CAPACITY - used {
                    return Err(IO);
                }
                used += read;
                if used > LIMIT {
                    return Err(TEXT_LIMIT);
                }
                if read == 0 {
                    break;
                }
            }
            core::str::from_utf8(&output[..used]).map_err(|_| TEXT)?;
            Ok(DirectoryResult {
                output_len: used as u64,
                entry_count: 0,
            })
        } else {
            call(77, fd, 0, 0, 0)?;
            let input =
                unsafe { core::slice::from_raw_parts(request.source, request.source_len as usize) };
            let mut used = 0;
            while used < input.len() {
                let written = call(
                    1,
                    fd,
                    input[used..].as_ptr() as usize,
                    input.len() - used,
                    0,
                )?;
                if written == 0 || written > input.len() - used {
                    return Err(IO);
                }
                used += written;
            }
            Ok(DirectoryResult {
                output_len: 0,
                entry_count: 0,
            })
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Name {
    offset: usize,
    len: usize,
}
fn listing(fd: usize, scratch: &mut [u8], output: &mut [u8]) -> Result<DirectoryResult, u64> {
    let (first, rest) = scratch.split_at_mut(LIMIT);
    let (second, rest) = rest.split_at_mut(LIMIT);
    let (names, rest) = rest.split_at_mut(LIMIT);
    let records = &mut rest[..65_536];
    // Caller supplies aligned scratch; each of these disjoint regions is 1 MiB.
    let first =
        unsafe { core::slice::from_raw_parts_mut(first.as_mut_ptr().cast::<Name>(), COUNT) };
    let second =
        unsafe { core::slice::from_raw_parts_mut(second.as_mut_ptr().cast::<Name>(), COUNT) };
    let mut count = 0;
    let mut packed = 0;
    loop {
        let size = call(217, fd, records.as_mut_ptr() as usize, records.len(), 0)?;
        if size > records.len() {
            return Err(IO);
        }
        if size == 0 {
            break;
        }
        let mut at = 0;
        while at < size {
            if size - at < 20 {
                return Err(IO);
            }
            let len = u16::from_ne_bytes([records[at + 16], records[at + 17]]) as usize;
            if len < 20 || !len.is_multiple_of(8) || len > size - at {
                return Err(IO);
            }
            let tail = &records[at + 19..at + len];
            let end = tail.iter().position(|b| *b == 0).ok_or(IO)?;
            let name = &tail[..end];
            if name.is_empty() || name.contains(&b'/') {
                return Err(IO);
            }
            if name != b"." && name != b".." {
                if count == COUNT || name.len() > LIMIT - packed {
                    return Err(LIST_LIMIT);
                }
                first[count] = Name {
                    offset: packed,
                    len: name.len(),
                };
                names[packed..packed + name.len()].copy_from_slice(name);
                packed += name.len();
                count += 1;
            }
            at += len;
        }
    }
    encode_names(
        &mut first[..count],
        &mut second[..count],
        &names[..packed],
        output,
    )
}
fn bytes(name: Name, names: &[u8]) -> &[u8] {
    &names[name.offset..name.offset + name.len]
}
fn encode_names<'a>(
    mut first: &'a mut [Name],
    mut second: &'a mut [Name],
    names: &[u8],
    output: &mut [u8],
) -> Result<DirectoryResult, u64> {
    for name in first.iter() {
        core::str::from_utf8(bytes(*name, names)).map_err(|_| TEXT)?;
    }
    let count = first.len();
    let mut width = 1;
    while width < count {
        let mut start = 0;
        while start < count {
            let middle = (start + width).min(count);
            let end = (middle + width).min(count);
            let (mut left, mut right) = (start, middle);
            for target in &mut second[start..end] {
                if left < middle
                    && (right == end || bytes(first[left], names) <= bytes(first[right], names))
                {
                    *target = first[left];
                    left += 1;
                } else {
                    *target = first[right];
                    right += 1;
                }
            }
            start = end;
        }
        core::mem::swap(&mut first, &mut second);
        width *= 2;
    }
    let mut writer = Encoder { output, used: 0 };
    writer.put(b"[")?;
    for (i, name) in first.iter().enumerate() {
        if i != 0 {
            writer.put(b",")?;
        }
        writer.put(b"\"")?;
        // Full collection was validated before sorting/canonical size checks.
        let text = core::str::from_utf8(bytes(*name, names)).map_err(|_| TEXT)?;
        for ch in text.chars() {
            match ch {
                '"' => writer.put(b"\\\"")?,
                '\\' => writer.put(b"\\\\")?,
                '\n' => writer.put(b"\\n")?,
                '\r' => writer.put(b"\\r")?,
                '\t' => writer.put(b"\\t")?,
                ch if ch.is_control() => {
                    let n = ch as usize;
                    const HEX: &[u8; 16] = b"0123456789abcdef";
                    writer.put(&[b'\\', b'u', b'0', b'0', HEX[(n >> 4) & 15], HEX[n & 15]])?;
                }
                _ => {
                    let mut encoded = [0; 4];
                    writer.put(ch.encode_utf8(&mut encoded).as_bytes())?;
                }
            }
        }
        writer.put(b"\"")?;
    }
    writer.put(b"]")?;
    Ok(DirectoryResult {
        output_len: writer.used as u64,
        entry_count: count as u64,
    })
}
struct Encoder<'a> {
    output: &'a mut [u8],
    used: usize,
}
impl Encoder<'_> {
    fn put(&mut self, bytes: &[u8]) -> Result<(), u64> {
        if bytes.len() > LIMIT - self.used {
            return Err(LIST_LIMIT);
        }
        self.output[self.used..self.used + bytes.len()].copy_from_slice(bytes);
        self.used += bytes.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CString, OsString};
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "ail-directory-host-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn run(
            &self,
            kind: u64,
            path: Option<&[u8]>,
            source: Option<&[u8]>,
        ) -> (u64, DirectoryResult, Vec<u8>) {
            let root = CString::new(self.0.as_os_str().as_encoded_bytes()).unwrap();
            let mut output = vec![0; 1_048_577];
            let mut scratch = vec![0_u64; 0x311000 / 8];
            let mut request = request(kind, path, source);
            request.root = root.as_ptr().cast();
            request.output = output.as_mut_ptr();
            request.output_capacity = output.len() as u64;
            request.scratch = scratch.as_mut_ptr().cast();
            request.scratch_capacity = 0x311000;
            let mut result = DirectoryResult {
                output_len: 99,
                entry_count: 77,
            };
            let status = unsafe { ail_directory_run(&request, &mut result) };
            if status == 0 {
                output.truncate(result.output_len as usize);
            } else {
                assert_eq!(
                    result,
                    DirectoryResult {
                        output_len: 99,
                        entry_count: 77
                    }
                );
            }
            (status, result, output)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn request(kind: u64, path: Option<&[u8]>, source: Option<&[u8]>) -> DirectoryRequest {
        DirectoryRequest {
            kind,
            root: core::ptr::null(),
            path: path.map_or(core::ptr::null(), |v| v.as_ptr()),
            path_len: path.map_or(0, |v| v.len() as u64),
            source: source.map_or(core::ptr::null(), |v| v.as_ptr()),
            source_len: source.map_or(0, |v| v.len() as u64),
            output: core::ptr::null_mut(),
            output_capacity: 0,
            scratch: core::ptr::null_mut(),
            scratch_capacity: 0,
        }
    }

    #[test]
    fn directory_validation_precedes_root_and_allocation() {
        assert_eq!(core::mem::size_of::<DirectoryRequest>(), 80);
        assert_eq!(core::mem::size_of::<DirectoryResult>(), 16);
        for (kind, path, source, status) in [
            (0, Some(b"".as_slice()), None, 0),
            (1, Some(b"a/b".as_slice()), None, 0),
            (2, Some(b"a".as_slice()), Some(b"\0\n".as_slice()), 0),
            (1, None, None, 2),
            (1, Some(b"".as_slice()), None, 1),
            (2, Some(b"../x".as_slice()), None, 1),
            (2, Some(b"a".as_slice()), None, 2),
            (0, Some(b"\xff".as_slice()), None, 2),
        ] {
            assert_eq!(
                unsafe { ail_directory_validate(&request(kind, path, source)) },
                status,
                "{kind} {path:?}"
            );
        }
    }

    #[test]
    fn directory_lists_all_names_with_unicode_order_and_canonical_controls() {
        let fixture = Fixture::new();
        for name in ["z", ".hidden", "é", "🦀", "a\n\u{7f}\u{85}", "quote\"\\"] {
            std::fs::write(fixture.0.join(name), b"").unwrap();
        }
        std::fs::create_dir(fixture.0.join("folder")).unwrap();
        std::os::unix::fs::symlink("z", fixture.0.join("link")).unwrap();
        let (status, result, output) = fixture.run(0, Some(b""), None);
        assert_eq!(status, 0);
        assert_eq!(result.entry_count, 8);
        assert_eq!(output, "[\".hidden\",\"a\\n\\u007f\\u0085\",\"folder\",\"link\",\"quote\\\"\\\\\",\"z\",\"é\",\"🦀\"]".as_bytes());
    }

    #[test]
    fn directory_reads_writes_nested_files_and_rejects_symlink_escape() {
        let fixture = Fixture::new();
        let outside = Fixture::new();
        std::fs::create_dir(fixture.0.join("nested")).unwrap();
        std::fs::write(outside.0.join("sentinel"), b"safe").unwrap();
        std::os::unix::fs::symlink(&outside.0, fixture.0.join("link")).unwrap();
        let content = "é\0\r\n🦀".as_bytes();
        assert_eq!(fixture.run(2, Some(b"nested/file"), Some(content)).0, 0);
        assert_eq!(fixture.run(1, Some(b"nested/file"), None).2, content);
        assert_eq!(fixture.run(2, Some(b"link/sentinel"), Some(b"bad")).0, 1);
        assert_eq!(fixture.run(1, Some(b"link/sentinel"), None).0, 1);
        assert_eq!(std::fs::read(outside.0.join("sentinel")).unwrap(), b"safe");
        assert_eq!(fixture.run(1, Some(b"nested"), None).0, 1);
    }

    #[test]
    fn directory_invalid_names_and_read_limit_have_exact_statuses() {
        let fixture = Fixture::new();
        std::fs::write(fixture.0.join(OsString::from_vec(vec![0xff])), b"").unwrap();
        assert_eq!(fixture.run(0, Some(b""), None).0, 2);
        std::fs::write(fixture.0.join("bad"), b"\xff").unwrap();
        assert_eq!(fixture.run(1, Some(b"bad"), None).0, 2);
        std::fs::write(fixture.0.join("bad"), vec![0xff; 1_048_577]).unwrap();
        assert_eq!(fixture.run(1, Some(b"bad"), None).0, 3);
    }

    #[test]
    fn directory_validation_bounds_and_literal_components() {
        for path in [b"/a".as_slice(), b"a/", b"a//b", b"./a", b"a/../b", b"a\0b"] {
            assert_eq!(
                unsafe { ail_directory_validate(&request(2, Some(path), None)) },
                PATH
            );
        }
        for path in [".hidden", "a/..literal", "é/🦀", "a\\b", "a =\n\t"] {
            assert_eq!(
                unsafe { ail_directory_validate(&request(1, Some(path.as_bytes()), None)) },
                0
            );
        }
        for (size, status) in [
            (4095, 0),
            (4096, PATH),
            (LIMIT, PATH),
            (LIMIT + 1, TEXT_LIMIT),
        ] {
            let path = vec![b'x'; size];
            assert_eq!(
                unsafe { ail_directory_validate(&request(1, Some(&path), None)) },
                status
            );
        }
        let invalid = vec![0xff; 4096];
        assert_eq!(
            unsafe { ail_directory_validate(&request(1, Some(&invalid), None)) },
            TEXT
        );
        let source = vec![0xff; LIMIT + 1];
        assert_eq!(
            unsafe { ail_directory_validate(&request(2, Some(b"a"), Some(&source))) },
            TEXT_LIMIT
        );
        let fixture = Fixture::new();
        std::fs::write(fixture.0.join("a"), b"original").unwrap();
        assert_eq!(fixture.run(2, Some(b"a"), Some(b"\xff")).0, TEXT);
        assert_eq!(std::fs::read(fixture.0.join("a")).unwrap(), b"original");
    }

    #[test]
    fn directory_count_limit_exactly_fills_descriptor_scratch() {
        let fixture = Fixture::new();
        for i in (0..COUNT).rev() {
            std::fs::write(fixture.0.join(format!("{i:05}")), b"").unwrap();
        }
        let (status, result, output) = fixture.run(0, Some(b""), None);
        assert_eq!(status, 0);
        assert_eq!(result.entry_count, COUNT as u64);
        let expected = format!(
            "[{}]",
            (0..COUNT)
                .map(|i| format!("\"{i:05}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(output, expected.as_bytes());
        std::fs::write(fixture.0.join("extra"), b"").unwrap();
        assert_eq!(fixture.run(0, Some(b""), None).0, LIST_LIMIT);
    }

    #[test]
    fn directory_raw_bound_precedes_utf8_and_canonical_expansion() {
        let fixture = Fixture::new();
        for i in 0..8192 {
            let mut name = format!("{i:04}{}", "x".repeat(124)).into_bytes();
            if i == 0 {
                name[127] = 0xff;
            }
            std::fs::write(fixture.0.join(OsString::from_vec(name)), b"").unwrap();
        }
        // Exactly 1 MiB raw storage remains admissible; UTF-8 wins over encoding.
        assert_eq!(fixture.run(0, Some(b""), None).0, TEXT);
        std::fs::write(fixture.0.join("a"), b"").unwrap();
        assert_eq!(fixture.run(0, Some(b""), None).0, LIST_LIMIT);
        std::fs::remove_file(fixture.0.join("a")).unwrap();
        let mut bad = format!("0000{}", "x".repeat(124)).into_bytes();
        bad[127] = 0xff;
        std::fs::rename(
            fixture.0.join(OsString::from_vec(bad)),
            fixture.0.join(format!("0000{}", "x".repeat(124))),
        )
        .unwrap();
        assert_eq!(fixture.run(0, Some(b""), None).0, LIST_LIMIT);
    }

    #[test]
    fn directory_canonical_limit_and_all_merge_pass_parities() {
        let fixture = Fixture::new();
        for count in [0, 1, 3, 5, 17] {
            let directory = fixture.0.join(format!("n{count}"));
            std::fs::create_dir(&directory).unwrap();
            for i in (0..count).rev() {
                std::fs::write(directory.join(format!("{i:02}")), b"").unwrap();
            }
            let (status, result, output) =
                fixture.run(0, Some(format!("n{count}").as_bytes()), None);
            assert_eq!(status, 0);
            assert_eq!(result.entry_count, count);
            assert_eq!(
                output,
                format!(
                    "[{}]",
                    (0..count)
                        .map(|i| format!("\"{i:02}\""))
                        .collect::<Vec<_>>()
                        .join(",")
                )
                .as_bytes()
            );
        }
        let expansion = fixture.0.join("expansion");
        std::fs::create_dir(&expansion).unwrap();
        for i in 0..1024 {
            std::fs::write(
                expansion.join(format!("{i:04}{}", "\u{1}".repeat(180))),
                b"",
            )
            .unwrap();
        }
        assert_eq!(fixture.run(0, Some(b"expansion"), None).0, LIST_LIMIT);
        let bytes = vec![b'x'; LIMIT];
        assert_eq!(fixture.run(2, Some(b"full"), Some(&bytes)).0, 0);
        assert_eq!(fixture.run(1, Some(b"full"), None).2, bytes);
        assert_eq!(fixture.run(2, Some(b"full"), Some(b"")).0, 0);
        assert!(fixture.run(1, Some(b"full"), None).2.is_empty());
    }

    #[test]
    fn directory_regular_type_root_symlink_and_descriptor_cleanup() {
        let fixture = Fixture::new();
        std::os::unix::fs::symlink(".", fixture.0.join("root-link")).unwrap();
        std::fs::write(fixture.0.join("value"), b"hello").unwrap();
        let linked = Fixture(fixture.0.join("root-link"));
        assert_eq!(linked.run(1, Some(b"value"), None).2, b"hello");
        drop(linked); // remove_dir_all removes this symlink itself without following it.
        let fifo = CString::new(fixture.0.join("fifo").as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(
            syscall(133, fifo.as_ptr() as usize, 0o010600, 0, 0, 0, 0),
            0
        );
        for _ in 0..128 {
            assert_eq!(fixture.run(1, Some(b"fifo"), None).0, PATH);
            assert_eq!(fixture.run(2, Some(b"fifo"), Some(b"x")).0, PATH);
            assert_eq!(fixture.run(1, Some(b"missing"), None).0, PATH);
        }
        for fd in std::fs::read_dir("/proc/self/fd").unwrap() {
            if let Ok(target) = std::fs::read_link(fd.unwrap().path()) {
                assert!(
                    !target.starts_with(&fixture.0),
                    "leaked descriptor to {target:?}"
                );
            }
        }
    }
}
