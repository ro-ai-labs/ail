#![no_std]
#![no_main]

#[path = "../src/float_number.rs"]
#[allow(dead_code)]
mod float_number;
use float_number::Error;

#[path = "../src/byte_engine.rs"]
#[allow(dead_code)]
mod byte_engine;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_bytes_validate(request: *const byte_engine::ByteRequest) -> u64 {
    unsafe { byte_engine::validate(request) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_bytes_run(
    request: *const byte_engine::ByteRequest,
    result: *mut byte_engine::ByteResult,
) -> u64 {
    unsafe { byte_engine::run(request, result) }
}

#[path = "../src/process_host.rs"]
mod process_host;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_process_run(
    request: *const process_host::ProcessRequest,
    result: *mut process_host::ProcessResult,
) -> u64 {
    unsafe { process_host::ail_process_run(request, result) }
}

#[path = "../src/directory_host.rs"]
mod directory_host;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_directory_validate(
    request: *const directory_host::DirectoryRequest,
) -> u64 {
    unsafe { directory_host::ail_directory_validate(request) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_directory_run(
    request: *const directory_host::DirectoryRequest,
    result: *mut directory_host::DirectoryResult,
) -> u64 {
    unsafe { directory_host::ail_directory_run(request, result) }
}

#[path = "../src/value_engine.rs"]
mod value_engine;
#[path = "../src/value_leaf.rs"]
mod value_leaf;
#[path = "../src/value_transport.rs"]
mod value_transport;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_value_run(
    request: *const value_engine::ValueRequest,
    result: *mut value_engine::ValueResult,
) -> u64 {
    unsafe { value_engine::run(request, result) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_value_schema_check(schema: *const u8, len: u64) -> u64 {
    unsafe { value_engine::schema_check(schema, len) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_value_validate_argument(
    request: *const value_engine::ValueRequest,
) -> u64 {
    unsafe { value_engine::validate_argument(request) }
}

// All exported functions use integer bits and explicit output pointers. AIL's
// generated caller supplies valid buffers and aligns RSP to the SysV call ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_float_parse(data: *const u8, length: usize, out: *mut u64) -> usize {
    unsafe { parse(data, length, out, false) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_float_parse_json(
    data: *const u8,
    length: usize,
    out: *mut u64,
) -> usize {
    unsafe { parse(data, length, out, true) }
}
unsafe fn parse(data: *const u8, length: usize, out: *mut u64, json: bool) -> usize {
    if length > float_number::INPUT_LIMIT {
        return Error::Limit.status();
    }
    if data.is_null() || out.is_null() {
        return Error::Invalid.status();
    }
    let bytes = unsafe { core::slice::from_raw_parts(data, length) };
    let Ok(text) = core::str::from_utf8(bytes) else {
        return Error::Invalid.status();
    };
    let parsed = if json {
        float_number::parse_json(text)
    } else {
        float_number::parse(text)
    };
    match parsed {
        Ok(bits) => {
            unsafe {
                out.write(bits);
            }
            0
        }
        Err(error) => error.status(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_float_format(bits: u64, data: *mut u8, capacity: usize) -> usize {
    if data.is_null() || capacity > isize::MAX as usize {
        return usize::MAX;
    }
    let output = unsafe { core::slice::from_raw_parts_mut(data, capacity) };
    float_number::format(bits, output).unwrap_or(usize::MAX)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_float_binary(op: u8, left: u64, right: u64, out: *mut u64) -> usize {
    if out.is_null() {
        return Error::Buffer.status();
    }
    match float_number::binary(op, left, right) {
        Ok(bits) => {
            unsafe {
                out.write(bits);
            }
            0
        }
        Err(error) => error.status(),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn ail_float_compare(op: u8, left: u64, right: u64) -> usize {
    usize::from(float_number::compare(op, left, right))
}
#[unsafe(no_mangle)]
pub extern "C" fn ail_float_from_integer(value: i64) -> u64 {
    float_number::from_integer(value)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ail_float_to_integer(bits: u64, out: *mut i64) -> usize {
    if out.is_null() {
        return Error::Buffer.status();
    }
    match float_number::to_integer(bits) {
        Ok(value) => {
            unsafe {
                out.write(value);
            }
            0
        }
        Err(error) => error.status(),
    }
}

// Core formatting and decimal parsing may use these compiler memory intrinsics.
// Volatile accesses stop LLVM from turning their definitions into self-calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, length: usize) -> *mut u8 {
    for index in 0..length {
        unsafe {
            dst.add(index)
                .write_volatile(src.add(index).read_volatile());
        }
    }
    dst
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dst: *mut u8, value: i32, length: usize) -> *mut u8 {
    for index in 0..length {
        unsafe {
            dst.add(index).write_volatile(value as u8);
        }
    }
    dst
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, length: usize) -> *mut u8 {
    if dst as usize <= src as usize {
        unsafe { memcpy(dst, src, length) }
    } else {
        for index in (0..length).rev() {
            unsafe {
                dst.add(index)
                    .write_volatile(src.add(index).read_volatile());
            }
        }
        dst
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(left: *const u8, right: *const u8, length: usize) -> i32 {
    for index in 0..length {
        let (left, right) = unsafe {
            (
                left.add(index).read_volatile(),
                right.add(index).read_volatile(),
            )
        };
        if left != right {
            return i32::from(left) - i32::from(right);
        }
    }
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(left: *const u8, right: *const u8, length: usize) -> i32 {
    unsafe { memcmp(left, right, length) }
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    unsafe {
        core::arch::asm!("syscall", in("rax") 60usize, in("rdi") 90usize, options(noreturn));
    }
}
// This embedded image is not entered directly by an AIL executable. Retaining a
// harmless entry point also makes accidental direct execution fail explicitly.
core::arch::global_asm!(
    ".global ail_float_runtime_start",
    "ail_float_runtime_start:",
    "and rsp, -16",
    "mov eax, 60",
    "mov edi, 90",
    "syscall",
    "ud2",
);
