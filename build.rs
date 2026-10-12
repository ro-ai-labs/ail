//! Build the fixed-address no_std numeric runtime once, while building AIL.
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const EXPORTS: [&str; 15] = [
    "ail_float_parse",
    "ail_float_parse_json",
    "ail_float_format",
    "ail_float_binary",
    "ail_float_compare",
    "ail_float_from_integer",
    "ail_float_to_integer",
    "ail_process_run",
    "ail_directory_validate",
    "ail_directory_run",
    "ail_bytes_validate",
    "ail_bytes_run",
    "ail_value_run",
    "ail_value_schema_check",
    "ail_value_validate_argument",
];
const BASE: u64 = 0x1000_0000;
const LIMIT: u64 = 0x2000_0000;

fn main() {
    println!("cargo:rerun-if-changed=runtime/float_runtime.rs");
    println!("cargo:rerun-if-changed=src/float_number.rs");
    println!("cargo:rerun-if-changed=src/process_host.rs");
    println!("cargo:rerun-if-changed=src/directory_host.rs");
    println!("cargo:rerun-if-changed=src/byte_engine.rs");
    println!("cargo:rerun-if-changed=src/value_transport.rs");
    println!("cargo:rerun-if-changed=src/value_leaf.rs");
    println!("cargo:rerun-if-changed=src/value_engine.rs");
    println!("cargo:rerun-if-changed=build.rs");
    if let Err(error) = build() {
        panic!("cannot build embedded Float runtime: {error}");
    }
}
fn build() -> Result<(), String> {
    let root =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("missing CARGO_MANIFEST_DIR")?);
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let sysroot = Command::new(&rustc)
        .args(["--print", "sysroot"])
        .output()
        .map_err(|e| e.to_string())?;
    if !sysroot.status.success() {
        return Err("rustc could not report its sysroot".into());
    }
    let sysroot = PathBuf::from(
        String::from_utf8(sysroot.stdout)
            .map_err(|e| e.to_string())?
            .trim(),
    );
    let host = env::var("HOST").map_err(|e| e.to_string())?;
    let linker = sysroot.join("lib/rustlib").join(host).join("bin/rust-lld");
    if !linker.is_file() {
        return Err("the Rust toolchain does not contain rust-lld".into());
    }
    let script = out.join("float_runtime.ld");
    fs::write(&script, r#"ENTRY(ail_float_runtime_start)
PHDRS { text PT_LOAD FLAGS(5); rodata PT_LOAD FLAGS(4); data PT_LOAD FLAGS(6); stack PT_GNU_STACK FLAGS(6); }
SECTIONS {
 . = 0x10000000;
 .text : ALIGN(16) { *(.text .text.*) } :text
 . = ALIGN(4096);
 .rodata : { *(.rodata .rodata.*) *(.gcc_except_table .gcc_except_table.*) *(.eh_frame .eh_frame.*) } :rodata
 . = ALIGN(4096);
 .data : { *(.data .data.*) *(.data.rel.ro .data.rel.ro.*) *(.got .got.*) } :data
 .bss : { *(.bss .bss.*) *(COMMON) } :data
 /DISCARD/ : { *(.comment) *(.note*) *(.eh_frame_hdr) }
}
"#).map_err(|e| e.to_string())?;
    let binary = out.join("float_runtime.elf");
    let mut command = Command::new(rustc);
    command
        .arg(root.join("runtime/float_runtime.rs"))
        .args([
            "--edition=2024",
            "--crate-name=ail_float_runtime",
            "--crate-type=bin",
            "--target=x86_64-unknown-linux-gnu",
        ])
        .args([
            "-C",
            "panic=abort",
            "-C",
            "opt-level=2",
            "-C",
            "codegen-units=1",
            "-C",
            "relocation-model=static",
            "-C",
            "linker-flavor=ld.lld",
        ])
        .arg("-C")
        .arg(format!("linker={}", linker.display()))
        .arg("-C")
        .arg(format!("link-arg=-T{}", script.display()))
        .args([
            "-C",
            "link-arg=--build-id=none",
            "-C",
            "link-arg=--no-dynamic-linker",
            "-C",
            "link-arg=-static",
        ])
        .arg(format!("--remap-path-prefix={}=/ail", root.display()))
        .arg(format!("--remap-path-prefix={}=/ail-build", out.display()))
        .arg("-o")
        .arg(&binary);
    for export in EXPORTS {
        command
            .arg("-C")
            .arg(format!("link-arg=--undefined={export}"));
    }
    let result = command.output().map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).into_owned());
    }
    let image = fs::read(binary).map_err(|e| e.to_string())?;
    let (segments, symbols) = inspect(&image)?;
    let mut metadata = String::from("const SEGMENTS: &[Segment] = &[\n");
    for segment in segments {
        metadata.push_str(&format!("Segment {{ offset: {}, address: {}, file_size: {}, memory_size: {}, flags: {}, alignment: {} }},\n",
            segment.offset, segment.address, segment.file_size, segment.memory_size, segment.flags, segment.alignment));
    }
    metadata.push_str("];\nconst SYMBOLS: &[(&str, u64)] = &[\n");
    for (name, address) in symbols {
        metadata.push_str(&format!("({name:?}, {address}),\n"));
    }
    metadata.push_str("];\n");
    fs::write(out.join("float_runtime_metadata.rs"), metadata).map_err(|e| e.to_string())
}

#[derive(Debug)]
struct Segment {
    offset: u64,
    address: u64,
    file_size: u64,
    memory_size: u64,
    flags: u32,
    alignment: u64,
}
fn bytes(image: &[u8], offset: u64, length: u64) -> Result<&[u8], String> {
    let end = offset.checked_add(length).ok_or("ELF range overflow")?;
    image
        .get(
            usize::try_from(offset).map_err(|_| "ELF offset too large")?
                ..usize::try_from(end).map_err(|_| "ELF range too large")?,
        )
        .ok_or_else(|| "ELF range exceeds image".into())
}
fn u16_at(image: &[u8], offset: u64) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        bytes(image, offset, 2)?.try_into().unwrap(),
    ))
}
fn u32_at(image: &[u8], offset: u64) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        bytes(image, offset, 4)?.try_into().unwrap(),
    ))
}
fn u64_at(image: &[u8], offset: u64) -> Result<u64, String> {
    Ok(u64::from_le_bytes(
        bytes(image, offset, 8)?.try_into().unwrap(),
    ))
}
fn inspect(image: &[u8]) -> Result<(Vec<Segment>, BTreeMap<String, u64>), String> {
    if bytes(image, 0, 7)? != b"\x7fELF\x02\x01\x01"
        || u16_at(image, 16)? != 2
        || u16_at(image, 18)? != 62
        || u32_at(image, 20)? != 1
        || u16_at(image, 52)? != 64
        || u16_at(image, 54)? != 56
    {
        return Err("runtime is not a little-endian x86-64 ET_EXEC ELF".into());
    }
    let phoff = u64_at(image, 32)?;
    let count = u16_at(image, 56)?;
    if count == 0 || count > 64 {
        return Err("invalid runtime program header count".into());
    }
    let mut segments = Vec::new();
    for index in 0..u64::from(count) {
        let base = phoff
            .checked_add(index * 56)
            .ok_or("program header range overflow")?;
        bytes(image, base, 56)?;
        let kind = u32_at(image, base)?;
        if matches!(kind, 2 | 3 | 7) {
            return Err("runtime requires dynamic linking, interpreter or TLS".into());
        }
        if kind != 1 {
            continue;
        }
        let segment = Segment {
            flags: u32_at(image, base + 4)?,
            offset: u64_at(image, base + 8)?,
            address: u64_at(image, base + 16)?,
            file_size: u64_at(image, base + 32)?,
            memory_size: u64_at(image, base + 40)?,
            alignment: u64_at(image, base + 48)?,
        };
        if segment.memory_size == 0 {
            continue;
        }
        let end = segment
            .address
            .checked_add(segment.memory_size)
            .ok_or("runtime address overflow")?;
        if segment.address < BASE
            || end > LIMIT
            || segment.file_size > segment.memory_size
            || segment.alignment < 4096
            || !segment.alignment.is_power_of_two()
            || segment.offset % segment.alignment != segment.address % segment.alignment
            || segment.flags & !7 != 0
            || segment.flags & 4 == 0
            || segment.flags & 3 == 3
        {
            return Err("invalid runtime load segment".into());
        }
        bytes(image, segment.offset, segment.file_size)?;
        segments.push(segment);
    }
    segments.sort_by_key(|segment| segment.address);
    if segments.is_empty() {
        return Err("runtime has no load segments".into());
    }
    for pair in segments.windows(2) {
        let end = (pair[0].address + pair[0].memory_size + 4095) & !4095;
        if end > pair[1].address & !4095 {
            return Err("runtime load segments overlap".into());
        }
    }
    let shoff = u64_at(image, 40)?;
    let shnum = u16_at(image, 60)?;
    if u16_at(image, 58)? != 64 || shnum == 0 {
        return Err("runtime has no ELF symbol sections".into());
    }
    bytes(image, shoff, u64::from(shnum) * 64)?;
    let mut symbols = BTreeMap::new();
    for index in 0..u64::from(shnum) {
        let section = shoff + index * 64;
        if u32_at(image, section + 4)? != 2 {
            continue;
        }
        let offset = u64_at(image, section + 24)?;
        let size = u64_at(image, section + 32)?;
        if u64_at(image, section + 56)? != 24 || size % 24 != 0 {
            return Err("invalid runtime symbol table".into());
        }
        let strings_index = u32_at(image, section + 40)?;
        if strings_index >= u32::from(shnum) {
            return Err("invalid runtime string table index".into());
        }
        let strings_section = shoff + u64::from(strings_index) * 64;
        let strings = bytes(
            image,
            u64_at(image, strings_section + 24)?,
            u64_at(image, strings_section + 32)?,
        )?;
        bytes(image, offset, size)?;
        for symbol in (offset..offset + size).step_by(24) {
            let name_offset = u32_at(image, symbol)? as usize;
            let rest = strings
                .get(name_offset..)
                .ok_or("invalid symbol name offset")?;
            let end = rest
                .iter()
                .position(|byte| *byte == 0)
                .ok_or("unterminated symbol name")?;
            let name = std::str::from_utf8(&rest[..end]).map_err(|_| "invalid symbol name")?;
            let section_index = u16_at(image, symbol + 6)?;
            let binding = bytes(image, symbol + 4, 1)?[0] >> 4;
            if !name.is_empty() && section_index == 0 && binding != 2 {
                return Err(format!("runtime has undefined symbol {name}"));
            }
            if !EXPORTS.contains(&name) {
                continue;
            }
            let address = u64_at(image, symbol + 8)?;
            if section_index == 0
                || !segments.iter().any(|s| {
                    s.flags & 1 != 0 && address >= s.address && address < s.address + s.file_size
                })
            {
                return Err(format!("runtime helper {name} is not executable"));
            }
            if symbols.insert(name.to_string(), address).is_some() {
                return Err(format!("duplicate runtime helper {name}"));
            }
        }
    }
    if symbols.len() != EXPORTS.len() {
        return Err("runtime is missing required helper symbols".into());
    }
    Ok((segments, symbols))
}
