//! Validated, fixed-address no_std numeric runtime embedded at Cargo build time.
//! AIL compilation copies bytes; it does not run a compiler, linker or subprocess.
const RUNTIME_BASE: u64 = 0x1000_0000;
const RUNTIME_LIMIT: u64 = 0x2000_0000;
const IMAGE_BASE: u64 = 0x0040_0000;
const CODE_OFFSET: usize = 120;
const PAGE: u64 = 4096;
const IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/float_runtime.elf"));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Segment {
    offset: u64,
    address: u64,
    file_size: u64,
    memory_size: u64,
    flags: u32,
    alignment: u64,
}
include!(concat!(env!("OUT_DIR"), "/float_runtime_metadata.rs"));

pub(super) fn symbol(name: &str) -> Result<u64, String> {
    SYMBOLS
        .iter()
        .find_map(|(candidate, address)| (*candidate == name).then_some(*address))
        .ok_or_else(|| format!("unknown embedded Float runtime helper '{name}'"))
}

/// Retain the emitter's existing 0x400078 code address. The expanded program
/// header table follows the generated code inside its first read/execute load.
pub(super) fn wrap(code: &[u8]) -> Result<Vec<u8>, String> {
    validate(IMAGE)?;
    let count = SEGMENTS
        .len()
        .checked_add(2)
        .ok_or("too many native ELF segments")?;
    let table = align_up(
        (CODE_OFFSET as u64)
            .checked_add(code.len() as u64)
            .ok_or("native code size overflow")?,
        8,
    )?;
    let first_size = table
        .checked_add((count as u64) * 56)
        .ok_or("native ELF header size overflow")?;
    validate_code_end(first_size)?;
    let first_size = usize::try_from(first_size).map_err(|_| "native ELF is too large")?;
    let mut output = vec![0u8; first_size];
    output[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    put16(&mut output, 16, 2);
    put16(&mut output, 18, 62);
    put32(&mut output, 20, 1);
    put64(&mut output, 24, IMAGE_BASE + CODE_OFFSET as u64);
    put64(&mut output, 32, table);
    put16(&mut output, 52, 64);
    put16(&mut output, 54, 56);
    put16(
        &mut output,
        56,
        u16::try_from(count).map_err(|_| "too many native ELF segments")?,
    );
    output[CODE_OFFSET..CODE_OFFSET + code.len()].copy_from_slice(code);
    let table = table as usize;
    header(
        &mut output,
        table,
        1,
        Segment {
            offset: 0,
            address: IMAGE_BASE,
            file_size: first_size as u64,
            memory_size: first_size as u64,
            flags: 5,
            alignment: PAGE,
        },
    );
    for (index, segment) in SEGMENTS.iter().copied().enumerate() {
        let offset = align_up(output.len() as u64, PAGE)? + segment.address % PAGE;
        let offset = usize::try_from(offset).map_err(|_| "native ELF offset is too large")?;
        output.resize(offset, 0);
        output.extend_from_slice(range(IMAGE, segment.offset, segment.file_size)?);
        header(
            &mut output,
            table + (index + 1) * 56,
            1,
            Segment {
                offset: offset as u64,
                ..segment
            },
        );
    }
    // Explicitly request a writable, non-executable stack.
    header(
        &mut output,
        table + (count - 1) * 56,
        0x6474_e551,
        Segment {
            offset: 0,
            address: 0,
            file_size: 0,
            memory_size: 0,
            flags: 6,
            alignment: 16,
        },
    );
    Ok(output)
}
fn validate_code_end(size: u64) -> Result<(), String> {
    let end = IMAGE_BASE
        .checked_add(size)
        .ok_or("native code address overflow")?;
    if align_up(end, PAGE)? > RUNTIME_BASE {
        return Err("native code overlaps the embedded Float runtime".into());
    }
    Ok(())
}
fn align_up(value: u64, alignment: u64) -> Result<u64, String> {
    value
        .checked_add(alignment - 1)
        .map(|end| end & !(alignment - 1))
        .ok_or_else(|| "native ELF alignment overflow".into())
}
fn header(output: &mut [u8], start: usize, kind: u32, segment: Segment) {
    put32(output, start, kind);
    put32(output, start + 4, segment.flags);
    put64(output, start + 8, segment.offset);
    put64(output, start + 16, segment.address);
    put64(output, start + 24, segment.address);
    put64(output, start + 32, segment.file_size);
    put64(output, start + 40, segment.memory_size);
    put64(output, start + 48, segment.alignment);
}
fn put16(output: &mut [u8], offset: usize, value: u16) {
    output[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn put32(output: &mut [u8], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn put64(output: &mut [u8], offset: usize, value: u64) {
    output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn range(image: &[u8], offset: u64, length: u64) -> Result<&[u8], String> {
    let end = offset
        .checked_add(length)
        .ok_or("embedded Float ELF range overflow")?;
    let start = usize::try_from(offset).map_err(|_| "embedded Float ELF offset is too large")?;
    let end = usize::try_from(end).map_err(|_| "embedded Float ELF range is too large")?;
    image
        .get(start..end)
        .ok_or_else(|| "embedded Float ELF range exceeds image".into())
}
fn get16(image: &[u8], offset: u64) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        range(image, offset, 2)?.try_into().unwrap(),
    ))
}
fn get32(image: &[u8], offset: u64) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        range(image, offset, 4)?.try_into().unwrap(),
    ))
}
fn get64(image: &[u8], offset: u64) -> Result<u64, String> {
    Ok(u64::from_le_bytes(
        range(image, offset, 8)?.try_into().unwrap(),
    ))
}

fn validate(image: &[u8]) -> Result<(), String> {
    if range(image, 0, 7)? != b"\x7fELF\x02\x01\x01"
        || get16(image, 16)? != 2
        || get16(image, 18)? != 62
        || get32(image, 20)? != 1
        || get16(image, 52)? != 64
        || get16(image, 54)? != 56
    {
        return Err("invalid embedded Float ELF architecture or header".into());
    }
    let count = get16(image, 56)?;
    if count == 0 || count > 64 {
        return Err("invalid embedded Float ELF segment count".into());
    }
    let table = get64(image, 32)?;
    range(image, table, u64::from(count) * 56)?;
    let mut segments = Vec::new();
    for index in 0..u64::from(count) {
        let base = table + index * 56;
        let kind = get32(image, base)?;
        if matches!(kind, 2 | 3 | 7) {
            return Err(
                "embedded Float runtime requires dynamic linking, interpreter or TLS".into(),
            );
        }
        if kind == 0x6474_e551 && get32(image, base + 4)? & 1 != 0 {
            return Err("embedded Float runtime requires an executable stack".into());
        }
        if kind != 1 {
            continue;
        }
        let segment = Segment {
            flags: get32(image, base + 4)?,
            offset: get64(image, base + 8)?,
            address: get64(image, base + 16)?,
            file_size: get64(image, base + 32)?,
            memory_size: get64(image, base + 40)?,
            alignment: get64(image, base + 48)?,
        };
        if segment.memory_size == 0 {
            continue;
        }
        let end = segment
            .address
            .checked_add(segment.memory_size)
            .ok_or("embedded Float runtime address overflow")?;
        if segment.address < RUNTIME_BASE
            || end > RUNTIME_LIMIT
            || segment.file_size > segment.memory_size
            || segment.alignment != PAGE
            || segment.offset % PAGE != segment.address % PAGE
            || segment.flags & !7 != 0
            || segment.flags & 4 == 0
            || segment.flags & 3 == 3
        {
            return Err("invalid embedded Float runtime load segment".into());
        }
        range(image, segment.offset, segment.file_size)?;
        segments.push(segment);
    }
    segments.sort_by_key(|segment| segment.address);
    for pair in segments.windows(2) {
        if align_up(pair[0].address + pair[0].memory_size, PAGE)? > pair[1].address & !(PAGE - 1) {
            return Err("embedded Float runtime segments overlap".into());
        }
    }
    if segments != SEGMENTS {
        return Err("embedded Float runtime metadata does not match its ELF".into());
    }
    for (_, address) in SYMBOLS {
        if !segments.iter().any(|segment| {
            segment.flags & 1 != 0
                && *address >= segment.address
                && *address < segment.address + segment.file_size
        }) {
            return Err("embedded Float runtime symbol is outside executable memory".into());
        }
    }
    Ok(())
}
#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    use std::sync::Mutex;
    static EXECUTION: Mutex<()> = Mutex::new(());

    fn run(code: &[u8]) -> std::process::Output {
        let _guard = EXECUTION.lock().unwrap_or_else(|error| error.into_inner());
        let file =
            std::env::temp_dir().join(format!("ail-float-runtime-test-{}", std::process::id()));
        std::fs::write(&file, wrap(code).unwrap()).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = Command::new(&file).output().unwrap();
        std::fs::remove_file(file).unwrap();
        assert!(output.status.success(), "{output:?}");
        output
    }
    fn entry() -> Vec<u8> {
        vec![0x48, 0x83, 0xe4, 0xf0, 0x48, 0x81, 0xec, 0x80, 0, 0, 0]
    }
    fn immediate(code: &mut Vec<u8>, register: u8, value: u64) {
        code.extend([0x48, 0xb8 + register]);
        code.extend(value.to_le_bytes());
    }
    fn call(code: &mut Vec<u8>, name: &str) {
        immediate(code, 0, symbol(name).unwrap());
        code.extend([0xff, 0xd0]);
    }
    fn output(code: &mut Vec<u8>, length: u32) {
        code.extend([0x48, 0x89, 0xe6, 0xba]);
        code.extend(length.to_le_bytes());
        code.extend([0xb8, 1, 0, 0, 0, 0xbf, 1, 0, 0, 0, 0x0f, 0x05]);
    }
    fn exit(code: &mut Vec<u8>) {
        code.extend([0xb8, 60, 0, 0, 0, 0x31, 0xff, 0x0f, 0x05]);
    }

    #[test]
    fn standalone_runtime_parses_exact_decimal_and_reports_status() {
        let text = b"1.00000000000000011102230246251565404236316680908203125";
        let mut code = entry();
        code.extend([0x48, 0x8d, 0x3d]);
        let patch = code.len();
        code.extend([0; 4]);
        immediate(&mut code, 6, text.len() as u64);
        code.extend([0x48, 0x89, 0xe2]);
        call(&mut code, "ail_float_parse");
        code.extend([0x48, 0x89, 0x44, 0x24, 8]);
        output(&mut code, 16);
        exit(&mut code);
        let displacement = code.len() as i32 - (patch as i32 + 4);
        code[patch..patch + 4].copy_from_slice(&displacement.to_le_bytes());
        code.extend(text);
        let result = run(&code);
        assert_eq!(&result.stdout[..8], &1.0f64.to_bits().to_le_bytes());
        assert_eq!(&result.stdout[8..], &0u64.to_le_bytes());
    }

    #[test]
    fn standalone_runtime_parses_strict_finite_json_and_keeps_output_on_error() {
        for (text, expected, status) in [
            ("-0", 1u64 << 63, 0u64),
            ("5e-324", 1, 0),
            ("1e-9999", 0, 0),
            ("+1", 123, 1),
            ("nan", 123, 1),
            ("1e9999", 123, 1),
        ] {
            let mut code = entry();
            immediate(&mut code, 0, 123);
            code.extend([0x48, 0x89, 0x04, 0x24]);
            code.extend([0x48, 0x8d, 0x3d]);
            let patch = code.len();
            code.extend([0; 4]);
            immediate(&mut code, 6, text.len() as u64);
            code.extend([0x48, 0x89, 0xe2]);
            call(&mut code, "ail_float_parse_json");
            code.extend([0x48, 0x89, 0x44, 0x24, 8]);
            output(&mut code, 16);
            exit(&mut code);
            let displacement = code.len() as i32 - (patch as i32 + 4);
            code[patch..patch + 4].copy_from_slice(&displacement.to_le_bytes());
            code.extend(text.as_bytes());
            let result = run(&code);
            assert_eq!(&result.stdout[..8], &expected.to_le_bytes(), "{text}");
            assert_eq!(&result.stdout[8..], &status.to_le_bytes(), "{text}");
        }
    }

    #[test]
    fn standalone_runtime_formats_subnormal_without_dynamic_dependencies() {
        let mut code = entry();
        immediate(&mut code, 7, 1); // rdi=smallest subnormal bits
        code.extend([0x48, 0x89, 0xe6]);
        immediate(&mut code, 2, 32);
        call(&mut code, "ail_float_format");
        output(&mut code, 6);
        exit(&mut code);
        let result = run(&code);
        assert_eq!(result.stdout, b"5e-324");
    }

    #[test]
    fn standalone_runtime_remainder_and_integer_conversion_use_bits_abi() {
        let mut code = entry();
        immediate(&mut code, 7, 4);
        immediate(&mut code, 6, (-7.0f64).to_bits());
        immediate(&mut code, 2, 3.0f64.to_bits());
        code.extend([0x48, 0x89, 0xe1]);
        call(&mut code, "ail_float_binary");
        code.extend([0x48, 0x8b, 0x3c, 0x24, 0x48, 0x89, 0xe6]);
        call(&mut code, "ail_float_to_integer");
        code.extend([0x48, 0x89, 0x44, 0x24, 8]);
        output(&mut code, 16);
        exit(&mut code);
        let result = run(&code);
        assert_eq!(&result.stdout[..8], &2i64.to_le_bytes());
        assert_eq!(&result.stdout[8..], &0u64.to_le_bytes());
    }
    #[test]
    fn embedded_elf_validation_rejects_corruption_and_mapping_overlap() {
        assert!(validate(IMAGE).is_ok());
        let table = get64(IMAGE, 32).unwrap() as usize;
        for mutate in [
            (18usize, 0u64, 2usize),
            (32, u64::MAX, 8),
            (table, 3, 4),
            (table + 4, 7, 4),
            (table + 8, u64::MAX - 1, 8),
            (table + 16, RUNTIME_BASE - PAGE, 8),
            (table + 32, u64::MAX, 8),
            (table + 40, u64::MAX, 8),
            (table + 48, 8, 8),
            (table + 56 + 16, RUNTIME_BASE, 8),
        ] {
            let mut image = IMAGE.to_vec();
            image[mutate.0..mutate.0 + mutate.2]
                .copy_from_slice(&mutate.1.to_le_bytes()[..mutate.2]);
            assert!(
                validate(&image).is_err(),
                "accepted corrupt ELF at {}",
                mutate.0
            );
        }
        assert!(validate_code_end(RUNTIME_BASE - IMAGE_BASE).is_ok());
        assert!(validate_code_end(RUNTIME_BASE - IMAGE_BASE + 1).is_err());
        assert!(validate_code_end(u64::MAX).is_err());
        assert!(symbol("unrecognized_helper").is_err());
    }

    #[test]
    fn emitted_elf_preserves_code_addresses_and_static_runtime_permissions() {
        let code = [0xb8, 60, 0, 0, 0, 0x31, 0xff, 0x0f, 0x05];
        let image = wrap(&code).unwrap();
        assert_eq!(&image[CODE_OFFSET..CODE_OFFSET + code.len()], &code);
        assert_eq!(get64(&image, 24).unwrap(), IMAGE_BASE + CODE_OFFSET as u64);
        let table = get64(&image, 32).unwrap();
        let count = get16(&image, 56).unwrap();
        assert!(table >= (CODE_OFFSET + code.len()) as u64);
        let mut loads = 0;
        for index in 0..u64::from(count) {
            let base = table + index * 56;
            let kind = get32(&image, base).unwrap();
            assert!(!matches!(kind, 2 | 3 | 7));
            if kind != 1 {
                continue;
            }
            loads += 1;
            let offset = get64(&image, base + 8).unwrap();
            let address = get64(&image, base + 16).unwrap();
            let flags = get32(&image, base + 4).unwrap();
            assert_eq!(offset % PAGE, address % PAGE);
            assert_ne!(flags & 3, 3);
            assert!(range(&image, offset, get64(&image, base + 32).unwrap()).is_ok());
        }
        assert_eq!(loads, SEGMENTS.len() + 1);
    }

    #[test]
    fn standalone_runtime_comparison_and_integer_rounding_use_integer_registers() {
        let mut code = entry();
        immediate(&mut code, 7, 9007199254740993);
        call(&mut code, "ail_float_from_integer");
        code.extend([0x48, 0x89, 0x04, 0x24]);
        immediate(&mut code, 7, 1);
        immediate(&mut code, 6, 0x7ff8_0000_0000_0000);
        immediate(&mut code, 2, 0x7ff8_0000_0000_0000);
        call(&mut code, "ail_float_compare");
        code.extend([0x48, 0x89, 0x44, 0x24, 8]);
        output(&mut code, 16);
        exit(&mut code);
        let result = run(&code);
        assert_eq!(
            &result.stdout[..8],
            &9007199254740992.0f64.to_bits().to_le_bytes()
        );
        assert_eq!(&result.stdout[8..], &1u64.to_le_bytes());
    }
    #[test]
    fn standalone_runtime_returns_stable_error_statuses_without_writing_results() {
        let mut code = entry();
        // A zero divisor must win even when the dividend is NaN.
        immediate(&mut code, 7, 3);
        immediate(&mut code, 6, 0x7ff8_0000_0000_0000);
        immediate(&mut code, 2, 1 << 63);
        code.extend([0x48, 0x8d, 0x4c, 0x24, 64]);
        call(&mut code, "ail_float_binary");
        code.extend([0x48, 0x89, 0x04, 0x24]);
        // Excessive length is checked before touching the supplied byte pointer.
        immediate(&mut code, 7, 1);
        immediate(&mut code, 6, 1_048_577);
        code.extend([0x48, 0x8d, 0x54, 0x24, 64]);
        call(&mut code, "ail_float_parse");
        code.extend([0x48, 0x89, 0x44, 0x24, 8]);
        immediate(&mut code, 7, 9223372036854775808.0f64.to_bits());
        code.extend([0x48, 0x8d, 0x74, 0x24, 64]);
        call(&mut code, "ail_float_to_integer");
        code.extend([0x48, 0x89, 0x44, 0x24, 16]);
        immediate(&mut code, 7, 1.5f64.to_bits());
        code.extend([0x48, 0x8d, 0x74, 0x24, 64]);
        immediate(&mut code, 2, 2);
        call(&mut code, "ail_float_format");
        code.extend([0x48, 0x89, 0x44, 0x24, 24]);
        output(&mut code, 32);
        exit(&mut code);
        let result = run(&code);
        let expected = [3u64, 2, 4, u64::MAX]
            .into_iter()
            .flat_map(u64::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(result.stdout, expected);
    }

    #[test]
    fn standalone_runtime_preserves_sysv_callee_saved_registers() {
        let mut code = entry();
        immediate(&mut code, 3, 0x1111);
        immediate(&mut code, 5, 0x2222);
        for (register, value) in [(12u8, 0x3333u64), (13, 0x4444), (14, 0x5555), (15, 0x6666)] {
            code.extend([0x49, 0xb8 + register - 8]);
            code.extend(value.to_le_bytes());
        }
        immediate(&mut code, 7, 0x7fef_ffff_ffff_ffff);
        code.extend([0x48, 0x8d, 0x74, 0x24, 64]);
        immediate(&mut code, 2, 32);
        call(&mut code, "ail_float_format");
        code.extend([0x48, 0x89, 0x1c, 0x24, 0x48, 0x89, 0x6c, 0x24, 8]);
        for (modrm, offset) in [(0x64, 16), (0x6c, 24), (0x74, 32), (0x7c, 40)] {
            code.extend([0x4c, 0x89, modrm, 0x24, offset]);
        }
        output(&mut code, 48);
        exit(&mut code);
        let result = run(&code);
        let expected = [0x1111u64, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666]
            .into_iter()
            .flat_map(u64::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(result.stdout, expected);
    }
}
