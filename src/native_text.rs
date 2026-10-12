//! UTF-8 scalar operations and a bounded immutable arena for native actions.
//! Helpers preserve RBX (current frame), RBP (root arena), and R12–R15.
//! The result arena is mmap'd lazily. Find uses a temporary, separately bounded
//! prefix table for linear-time search, and releases it before returning.
use super::*;

pub(super) fn fields(instruction: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    let operands: &[&str] = match instruction.opcode.as_str() {
        "TEXT_JOIN" => &["key", "left", "right"],
        "TEXT_LENGTH" | "TEXT_SLICE" => &["key", "source"],
        "TEXT_FIND" => &["key", "source", "needle"],
        "TEXT_SPLIT" | "TEXT_JOIN_LIST" => &["key", "source", "separator"],
        "TEXT_REPLACE" => &["key", "source", "needle", "replacement"],
        "TEXT_TRIM" => &["key", "source"],
        _ => return Ok(BTreeSet::new()),
    };
    let mut fields = BTreeSet::new();
    for operand in operands {
        let key = required(instruction, operand)?;
        if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
            return Err("native text operand must be a field key".into());
        }
        fields.insert(key.to_string());
    }
    if instruction.opcode == "TEXT_SLICE" {
        for operand in ["start", "end"] {
            fields.extend(IntExpr::from_postfix(required(instruction, operand)?)?.fields());
        }
    }
    Ok(fields)
}

fn validate(emitter: &mut Emitter, key: &str) {
    emitter.load_rdi(emitter.slot(key) + POINTER);
    emitter.load_rsi(emitter.slot(key) + LENGTH);
    emitter.code.emit_call_label("text_validate");
}

pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
) -> Result<(), String> {
    let key = required(instruction, "key")?;
    match instruction.opcode.as_str() {
        "TEXT_SPLIT" => {
            validate(emitter, required(instruction, "source")?);
            emitter.code.emit(&[0x57, 0x56]);
            validate(emitter, required(instruction, "separator")?);
            emitter.code.emit(&[0x48, 0x85, 0xf6]); // nonempty separator
            emitter.code.emit_jcc_label(&[0x0f, 0x84], "error_text");
            emitter
                .code
                .emit(&[0x49, 0x89, 0xf8, 0x49, 0x89, 0xf1, 0x5e, 0x5f]);
            emitter.code.emit_call_label("tt_split");
        }
        "TEXT_JOIN_LIST" => {
            let source = emitter.slot(required(instruction, "source")?);
            emitter.load_rdi(source + POINTER);
            emitter.load_rsi(source + LENGTH);
            emitter.code.emit(&[0xba, 1, 0, 0, 0]);
            emitter.code.emit_call_label("list_parse");
            emitter.code.emit(&[0x50]); // parsed scratch survives separator validation
            validate(emitter, required(instruction, "separator")?);
            emitter
                .code
                .emit(&[0x49, 0x89, 0xf8, 0x49, 0x89, 0xf1, 0x5f]);
            emitter.code.emit_call_label("tt_join_list");
        }
        "TEXT_REPLACE" => {
            validate(emitter, required(instruction, "source")?);
            emitter.code.emit(&[0x57, 0x56]);
            validate(emitter, required(instruction, "needle")?);
            emitter.code.emit(&[0x57, 0x56]);
            validate(emitter, required(instruction, "replacement")?);
            emitter.code.emit(&[
                0x48, 0x89, 0xfa, 0x48, 0x89, 0xf1, 0x41, 0x59, 0x41, 0x58, 0x5e, 0x5f,
            ]);
            emitter.code.emit_call_label("tt_replace");
        }
        "TEXT_TRIM" => {
            validate(emitter, required(instruction, "source")?);
            emitter.code.emit_call_label("tt_trim");
        }
        "TEXT_LENGTH" => {
            validate(emitter, required(instruction, "source")?);
            emitter.store_integer(key);
            return Ok(());
        }
        "TEXT_JOIN" | "TEXT_FIND" => {
            let (left, right, helper) = if instruction.opcode == "TEXT_JOIN" {
                ("left", "right", "text_join")
            } else {
                ("source", "needle", "text_find")
            };
            validate(emitter, required(instruction, left)?);
            emitter.code.emit(&[0x57, 0x56]); // push left pointer, length
            validate(emitter, required(instruction, right)?);
            emitter
                .code
                .emit(&[0x49, 0x89, 0xf8, 0x49, 0x89, 0xf1, 0x5e, 0x5f]); // right -> r8/r9; pop left rsi/rdi
            emitter.code.emit_call_label(helper);
            if instruction.opcode == "TEXT_FIND" {
                emitter.store_integer(key);
                return Ok(());
            }
        }
        "TEXT_SLICE" => {
            validate(emitter, required(instruction, "source")?);
            emitter.code.emit(&[0x57, 0x56, 0x50]); // source pointer, bytes, scalar count
            emitter.expression(&IntExpr::from_postfix(required(instruction, "start")?)?)?;
            emitter.code.emit(&[0x50]); // start
            emitter.expression(&IntExpr::from_postfix(required(instruction, "end")?)?)?;
            emitter
                .code
                .emit(&[0x49, 0x89, 0xc1, 0x41, 0x58, 0x5a, 0x5e, 0x5f]); // end -> r9; pop start r8, count rdx, bytes rsi, pointer rdi
            emitter.code.emit_call_label("text_slice");
        }
        _ => return Err("unsupported native text operation".into()),
    }
    // All operands have been read and the immutable result is complete before
    // changing a destination, including destinations that alias an operand.
    let slot = emitter.slot(key);
    emitter.store_rax(slot + POINTER);
    emitter.store_rdx(slot + LENGTH);
    emitter.set(slot + NUMERIC, 0);
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    // RDI/RSI is a pointer/byte-length pair. Preserve it and return the scalar
    // count in RAX. Reject overlong encodings, surrogates and values > U+10FFFF.
    code.label("text_validate")?;
    code.emit(&[0x48, 0x85, 0xff]); // test   rdi,rdi
    code.emit_jcc_label(&[0x0f, 0x84], "error_text");
    code.emit(&[0x48, 0x81, 0xfe, 0x00, 0x00, 0x10, 0x00]); // cmp    rsi,0x100000
    code.emit_jcc_label(&[0x0f, 0x87], "error_text_limit");
    code.emit(&[0x49, 0x89, 0xf8]); // mov    r8,rdi
    code.emit(&[0x4c, 0x8d, 0x0c, 0x37]); // lea    r9,[rdi+rsi*1]
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.label("text_validate_loop")?;
    code.emit(&[0x4d, 0x39, 0xc8]); // cmp    r8,r9
    code.emit_jcc_label(&[0x0f, 0x84], "text_validate_done");
    code.emit(&[0x41, 0x0f, 0xb6, 0x08]); // movzx  ecx,BYTE PTR [r8]
    code.emit(&[0x49, 0xff, 0xc0]); // inc    r8
    code.emit(&[0x48, 0xff, 0xc0]); // inc    rax
    code.emit(&[0x81, 0xf9, 0x80, 0x00, 0x00, 0x00]); // cmp    ecx,0x80
    code.emit_jcc_label(&[0x0f, 0x82], "text_validate_loop");
    code.emit(&[0x81, 0xf9, 0xc2, 0x00, 0x00, 0x00]); // cmp    ecx,0xc2
    code.emit_jcc_label(&[0x0f, 0x82], "error_text");
    code.emit(&[0x81, 0xf9, 0xe0, 0x00, 0x00, 0x00]); // cmp    ecx,0xe0
    code.emit_jcc_label(&[0x0f, 0x82], "text_validate_two");
    code.emit(&[0x81, 0xf9, 0xf0, 0x00, 0x00, 0x00]); // cmp    ecx,0xf0
    code.emit_jcc_label(&[0x0f, 0x82], "text_validate_three");
    code.emit(&[0x81, 0xf9, 0xf5, 0x00, 0x00, 0x00]); // cmp    ecx,0xf5
    code.emit_jcc_label(&[0x0f, 0x83], "error_text");
    code.emit(&[0x4d, 0x39, 0xc8]); // cmp    r8,r9
    code.emit_jcc_label(&[0x0f, 0x83], "error_text");
    code.emit(&[0x41, 0x0f, 0xb6, 0x10]); // movzx  edx,BYTE PTR [r8]
    code.emit(&[0x81, 0xf9, 0xf0, 0x00, 0x00, 0x00]); // cmp    ecx,0xf0
    code.emit_jcc_label(&[0x0f, 0x85], "text_validate_four_max");
    code.emit(&[0x81, 0xfa, 0x90, 0x00, 0x00, 0x00]); // cmp    edx,0x90
    code.emit_jcc_label(&[0x0f, 0x82], "error_text");
    code.label("text_validate_four_max")?;
    code.emit(&[0x81, 0xf9, 0xf4, 0x00, 0x00, 0x00]); // cmp    ecx,0xf4
    code.emit_jcc_label(&[0x0f, 0x85], "text_validate_four_ready");
    code.emit(&[0x81, 0xfa, 0x90, 0x00, 0x00, 0x00]); // cmp    edx,0x90
    code.emit_jcc_label(&[0x0f, 0x83], "error_text");
    code.label("text_validate_four_ready")?;
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit_jmp_label("text_validate_continuation");
    code.label("text_validate_three")?;
    code.emit(&[0x4d, 0x39, 0xc8]); // cmp    r8,r9
    code.emit_jcc_label(&[0x0f, 0x83], "error_text");
    code.emit(&[0x41, 0x0f, 0xb6, 0x10]); // movzx  edx,BYTE PTR [r8]
    code.emit(&[0x81, 0xf9, 0xe0, 0x00, 0x00, 0x00]); // cmp    ecx,0xe0
    code.emit_jcc_label(&[0x0f, 0x85], "text_validate_three_max");
    code.emit(&[0x81, 0xfa, 0xa0, 0x00, 0x00, 0x00]); // cmp    edx,0xa0
    code.emit_jcc_label(&[0x0f, 0x82], "error_text");
    code.label("text_validate_three_max")?;
    code.emit(&[0x81, 0xf9, 0xed, 0x00, 0x00, 0x00]); // cmp    ecx,0xed
    code.emit_jcc_label(&[0x0f, 0x85], "text_validate_three_ready");
    code.emit(&[0x81, 0xfa, 0xa0, 0x00, 0x00, 0x00]); // cmp    edx,0xa0
    code.emit_jcc_label(&[0x0f, 0x83], "error_text");
    code.label("text_validate_three_ready")?;
    code.emit(&[0xba, 0x02, 0x00, 0x00, 0x00]); // mov    edx,0x2
    code.emit_jmp_label("text_validate_continuation");
    code.label("text_validate_two")?;
    code.emit(&[0xba, 0x01, 0x00, 0x00, 0x00]); // mov    edx,0x1
    code.label("text_validate_continuation")?;
    code.emit(&[0x4d, 0x39, 0xc8]); // cmp    r8,r9
    code.emit_jcc_label(&[0x0f, 0x83], "error_text");
    code.emit(&[0x41, 0x0f, 0xb6, 0x08]); // movzx  ecx,BYTE PTR [r8]
    code.emit(&[0x81, 0xe1, 0xc0, 0x00, 0x00, 0x00]); // and    ecx,0xc0
    code.emit(&[0x81, 0xf9, 0x80, 0x00, 0x00, 0x00]); // cmp    ecx,0x80
    code.emit_jcc_label(&[0x0f, 0x85], "error_text");
    code.emit(&[0x49, 0xff, 0xc0]); // inc    r8
    code.emit(&[0xff, 0xca]); // dec    edx
    code.emit_jcc_label(&[0x0f, 0x85], "text_validate_continuation");
    code.emit_jmp_label("text_validate_loop");
    code.label("text_validate_done")?;
    code.emit(&[0xc3]); // ret

    // RDX is the result size. Return its immutable address in RAX, preserving
    // RDX. RBP addresses the root's arena pointer and cumulative byte charge.
    // Validated sizes make both addition and max(size, 1) overflow-free.
    code.label("text_allocate")?;
    code.emit(&[0x48, 0x81, 0xfa, 0x00, 0x00, 0x10, 0x00]); // cmp    rdx,0x100000
    code.emit_jcc_label(&[0x0f, 0x87], "error_text_limit");
    code.emit(&[0x52]); // push   rdx
    code.emit(&[0x48, 0x89, 0xd1]); // mov    rcx,rdx
    code.emit(&[0x48, 0x85, 0xc9]); // test   rcx,rcx
    code.emit_jcc_label(&[0x0f, 0x85], "text_allocate_charge");
    code.emit(&[0xb9, 0x01, 0x00, 0x00, 0x00]); // mov    ecx,0x1
    code.label("text_allocate_charge")?;
    code.emit(&[0x48, 0x8b, 0x45, 0x08]); // mov    rax,QWORD PTR [rbp+0x8]
    code.emit(&[0x48, 0x01, 0xc8]); // add    rax,rcx
    code.emit(&[0x48, 0x3d, 0x00, 0x00, 0x00, 0x04]); // cmp    rax,0x4000000
    code.emit_jcc_label(&[0x0f, 0x87], "error_text_limit");
    code.emit(&[0x50]); // push   rax
    code.emit(&[0x48, 0x83, 0x7d, 0x00, 0x00]); // cmp    QWORD PTR [rbp+0x0],0x0
    code.emit_jcc_label(&[0x0f, 0x85], "text_allocate_ready");
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov    eax,0x9
    code.emit(&[0x31, 0xff]); // xor    edi,edi
    code.emit(&[0xbe, 0x00, 0x00, 0x00, 0x04]); // mov    esi,0x4000000
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov    r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov    r8,0xffffffffffffffff
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_text_allocation");
    code.emit(&[0x48, 0x89, 0x45, 0x00]); // mov    QWORD PTR [rbp+0x0],rax
    code.label("text_allocate_ready")?;
    code.emit(&[0x48, 0x8b, 0x45, 0x00]); // mov    rax,QWORD PTR [rbp+0x0]
    code.emit(&[0x48, 0x03, 0x45, 0x08]); // add    rax,QWORD PTR [rbp+0x8]
    code.emit(&[0x59]); // pop    rcx
    code.emit(&[0x48, 0x89, 0x4d, 0x08]); // mov    QWORD PTR [rbp+0x8],rcx
    code.emit(&[0x5a]); // pop    rdx
    code.emit(&[0xc3]); // ret

    // Validated left RDI/RSI and right R8/R9 -> result RAX/RDX. Save all source
    // descriptors before the allocator can clobber its syscall registers.
    code.label("text_join")?;
    code.emit(&[0x57]); // push   rdi
    code.emit(&[0x56]); // push   rsi
    code.emit(&[0x41, 0x50]); // push   r8
    code.emit(&[0x41, 0x51]); // push   r9
    code.emit(&[0x4a, 0x8d, 0x14, 0x0e]); // lea    rdx,[rsi+r9*1]
    code.emit_call_label("text_allocate");
    code.emit(&[0x49, 0x89, 0xc2]); // mov    r10,rax
    code.emit(&[0x49, 0x89, 0xd3]); // mov    r11,rdx
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit(&[0x48, 0x8b, 0x74, 0x24, 0x18]); // mov    rsi,QWORD PTR [rsp+0x18]
    code.emit(&[0x48, 0x8b, 0x4c, 0x24, 0x10]); // mov    rcx,QWORD PTR [rsp+0x10]
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0x48, 0x8b, 0x74, 0x24, 0x08]); // mov    rsi,QWORD PTR [rsp+0x8]
    code.emit(&[0x48, 0x8b, 0x0c, 0x24]); // mov    rcx,QWORD PTR [rsp]
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0x4c, 0x89, 0xd0]); // mov    rax,r10
    code.emit(&[0x4c, 0x89, 0xda]); // mov    rdx,r11
    code.emit(&[0x48, 0x83, 0xc4, 0x20]); // add    rsp,0x20
    code.emit(&[0xc3]); // ret

    // Source RDI/RSI, scalar count RDX, bounds R8/R9 -> result RAX/RDX.
    // Adding a nonnegative scalar count to a negative bound is safe even at
    // i64::MIN. Clamp first, then walk complete, previously validated scalars.
    code.label("text_slice")?;
    code.emit(&[0x4d, 0x85, 0xc0]); // test   r8,r8
    code.emit_jcc_label(&[0x0f, 0x89], "text_slice_start_nonnegative");
    code.emit(&[0x49, 0x01, 0xd0]); // add    r8,rdx
    code.label("text_slice_start_nonnegative")?;
    code.emit(&[0x4d, 0x85, 0xc0]); // test   r8,r8
    code.emit_jcc_label(&[0x0f, 0x89], "text_slice_start_max");
    code.emit(&[0x45, 0x31, 0xc0]); // xor    r8d,r8d
    code.label("text_slice_start_max")?;
    code.emit(&[0x49, 0x39, 0xd0]); // cmp    r8,rdx
    code.emit_jcc_label(&[0x0f, 0x8e], "text_slice_end");
    code.emit(&[0x49, 0x89, 0xd0]); // mov    r8,rdx
    code.label("text_slice_end")?;
    code.emit(&[0x4d, 0x85, 0xc9]); // test   r9,r9
    code.emit_jcc_label(&[0x0f, 0x89], "text_slice_end_nonnegative");
    code.emit(&[0x49, 0x01, 0xd1]); // add    r9,rdx
    code.label("text_slice_end_nonnegative")?;
    code.emit(&[0x4d, 0x85, 0xc9]); // test   r9,r9
    code.emit_jcc_label(&[0x0f, 0x89], "text_slice_end_max");
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.label("text_slice_end_max")?;
    code.emit(&[0x49, 0x39, 0xd1]); // cmp    r9,rdx
    code.emit_jcc_label(&[0x0f, 0x8e], "text_slice_order");
    code.emit(&[0x49, 0x89, 0xd1]); // mov    r9,rdx
    code.label("text_slice_order")?;
    code.emit(&[0x4d, 0x39, 0xc1]); // cmp    r9,r8
    code.emit_jcc_label(&[0x0f, 0x8d], "text_slice_scan_start");
    code.emit(&[0x4d, 0x89, 0xc1]); // mov    r9,r8
    code.label("text_slice_scan_start")?;
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.emit(&[0x31, 0xc9]); // xor    ecx,ecx
    code.emit(&[0x45, 0x31, 0xd2]); // xor    r10d,r10d
    code.label("text_slice_scan")?;
    code.emit(&[0x4c, 0x39, 0xc1]); // cmp    rcx,r8
    code.emit_jcc_label(&[0x0f, 0x85], "text_slice_check_end");
    code.emit(&[0x49, 0x89, 0xc2]); // mov    r10,rax
    code.label("text_slice_check_end")?;
    code.emit(&[0x4c, 0x39, 0xc9]); // cmp    rcx,r9
    code.emit_jcc_label(&[0x0f, 0x84], "text_slice_copy");
    code.emit(&[0x0f, 0xb6, 0x14, 0x07]); // movzx  edx,BYTE PTR [rdi+rax*1]
    code.emit(&[0x48, 0xff, 0xc0]); // inc    rax
    code.emit(&[0x81, 0xfa, 0x80, 0x00, 0x00, 0x00]); // cmp    edx,0x80
    code.emit_jcc_label(&[0x0f, 0x82], "text_slice_advanced");
    code.emit(&[0x48, 0xff, 0xc0]); // inc    rax
    code.emit(&[0x81, 0xfa, 0xe0, 0x00, 0x00, 0x00]); // cmp    edx,0xe0
    code.emit_jcc_label(&[0x0f, 0x82], "text_slice_advanced");
    code.emit(&[0x48, 0xff, 0xc0]); // inc    rax
    code.emit(&[0x81, 0xfa, 0xf0, 0x00, 0x00, 0x00]); // cmp    edx,0xf0
    code.emit_jcc_label(&[0x0f, 0x82], "text_slice_advanced");
    code.emit(&[0x48, 0xff, 0xc0]); // inc    rax
    code.label("text_slice_advanced")?;
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit_jmp_label("text_slice_scan");
    code.label("text_slice_copy")?;
    code.emit(&[0x4a, 0x8d, 0x34, 0x17]); // lea    rsi,[rdi+r10*1]
    code.emit(&[0x48, 0x89, 0xc2]); // mov    rdx,rax
    code.emit(&[0x4c, 0x29, 0xd2]); // sub    rdx,r10
    code.emit(&[0x56]); // push   rsi
    code.emit_call_label("text_allocate");
    code.emit(&[0x5e]); // pop    rsi
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit(&[0x48, 0x89, 0xd1]); // mov    rcx,rdx
    code.emit(&[0x50]); // push   rax
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0x58]); // pop    rax
    code.emit(&[0xc3]); // ret

    // Source RDI/RSI and needle R8/R9 -> scalar position RAX, or -1. KMP's
    // u32 prefix table uses at most 4 MiB and is freed on either search result.
    // A validated needle cannot begin inside a UTF-8 continuation sequence;
    // counting leading bytes before the match converts its offset to scalars.
    code.label("text_find")?;
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.emit(&[0x4d, 0x85, 0xc9]); // test   r9,r9
    code.emit_jcc_label(&[0x0f, 0x84], "text_find_trivial_done");
    code.emit(&[0x49, 0x39, 0xf1]); // cmp    r9,rsi
    code.emit_jcc_label(&[0x0f, 0x86], "text_find_allocate");
    code.emit(&[0x48, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov    rax,0xffffffffffffffff
    code.label("text_find_trivial_done")?;
    code.emit(&[0xc3]); // ret
    code.label("text_find_allocate")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x49, 0x89, 0xf5]); // mov    r13,rsi
    code.emit(&[0x4d, 0x89, 0xc6]); // mov    r14,r8
    code.emit(&[0x4d, 0x89, 0xcf]); // mov    r15,r9
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov    eax,0x9
    code.emit(&[0x31, 0xff]); // xor    edi,edi
    code.emit(&[0x4a, 0x8d, 0x34, 0xbd, 0x00, 0x00, 0x00, 0x00]); // lea    rsi,[r15*4+0x0]
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov    r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov    r8,0xffffffffffffffff
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit_call_label("recovery_temp_map");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_text_allocation");
    code.emit(&[0x48, 0x89, 0xc3]); // mov    rbx,rax
    code.emit(&[0xc7, 0x03, 0x00, 0x00, 0x00, 0x00]); // mov    DWORD PTR [rbx],0x0
    code.emit(&[0xb9, 0x01, 0x00, 0x00, 0x00]); // mov    ecx,0x1
    code.emit(&[0x31, 0xd2]); // xor    edx,edx
    code.label("text_find_prefix_loop")?;
    code.emit(&[0x4c, 0x39, 0xf9]); // cmp    rcx,r15
    code.emit_jcc_label(&[0x0f, 0x83], "text_find_search_start");
    code.emit(&[0x41, 0x8a, 0x04, 0x0e]); // mov    al,BYTE PTR [r14+rcx*1]
    code.label("text_find_prefix_compare")?;
    code.emit(&[0x41, 0x3a, 0x04, 0x16]); // cmp    al,BYTE PTR [r14+rdx*1]
    code.emit_jcc_label(&[0x0f, 0x84], "text_find_prefix_equal");
    code.emit(&[0x85, 0xd2]); // test   edx,edx
    code.emit_jcc_label(&[0x0f, 0x84], "text_find_prefix_store");
    code.emit(&[0x8b, 0x54, 0x93, 0xfc]); // mov    edx,DWORD PTR [rbx+rdx*4-0x4]
    code.emit_jmp_label("text_find_prefix_compare");
    code.label("text_find_prefix_equal")?;
    code.emit(&[0xff, 0xc2]); // inc    edx
    code.label("text_find_prefix_store")?;
    code.emit(&[0x89, 0x14, 0x8b]); // mov    DWORD PTR [rbx+rcx*4],edx
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit_jmp_label("text_find_prefix_loop");
    code.label("text_find_search_start")?;
    code.emit(&[0x31, 0xc9]); // xor    ecx,ecx
    code.emit(&[0x31, 0xd2]); // xor    edx,edx
    code.label("text_find_search_loop")?;
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp    rcx,r13
    code.emit_jcc_label(&[0x0f, 0x83], "text_find_not_found");
    code.emit(&[0x41, 0x8a, 0x04, 0x0c]); // mov    al,BYTE PTR [r12+rcx*1]
    code.label("text_find_search_compare")?;
    code.emit(&[0x41, 0x3a, 0x04, 0x16]); // cmp    al,BYTE PTR [r14+rdx*1]
    code.emit_jcc_label(&[0x0f, 0x84], "text_find_search_equal");
    code.emit(&[0x85, 0xd2]); // test   edx,edx
    code.emit_jcc_label(&[0x0f, 0x84], "text_find_search_next");
    code.emit(&[0x8b, 0x54, 0x93, 0xfc]); // mov    edx,DWORD PTR [rbx+rdx*4-0x4]
    code.emit_jmp_label("text_find_search_compare");
    code.label("text_find_search_equal")?;
    code.emit(&[0xff, 0xc2]); // inc    edx
    code.label("text_find_search_next")?;
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit(&[0x4c, 0x39, 0xfa]); // cmp    rdx,r15
    code.emit_jcc_label(&[0x0f, 0x85], "text_find_search_loop");
    code.emit(&[0x4c, 0x29, 0xf9]); // sub    rcx,r15
    code.emit(&[0x31, 0xd2]); // xor    edx,edx
    code.emit(&[0x45, 0x31, 0xd2]); // xor    r10d,r10d
    code.label("text_find_count_scalars")?;
    code.emit(&[0x48, 0x39, 0xca]); // cmp    rdx,rcx
    code.emit_jcc_label(&[0x0f, 0x83], "text_find_release");
    code.emit(&[0x41, 0x8a, 0x04, 0x14]); // mov    al,BYTE PTR [r12+rdx*1]
    code.emit(&[0x24, 0xc0]); // and    al,0xc0
    code.emit(&[0x3c, 0x80]); // cmp    al,0x80
    code.emit_jcc_label(&[0x0f, 0x84], "text_find_count_next");
    code.emit(&[0x49, 0xff, 0xc2]); // inc    r10
    code.label("text_find_count_next")?;
    code.emit(&[0x48, 0xff, 0xc2]); // inc    rdx
    code.emit_jmp_label("text_find_count_scalars");
    code.label("text_find_not_found")?;
    code.emit(&[0x49, 0xc7, 0xc2, 0xff, 0xff, 0xff, 0xff]); // mov    r10,0xffffffffffffffff
    code.label("text_find_release")?;
    code.emit(&[0xb8, 0x0b, 0x00, 0x00, 0x00]); // mov    eax,0xb
    code.emit(&[0x48, 0x89, 0xdf]); // mov    rdi,rbx
    code.emit(&[0x4a, 0x8d, 0x34, 0xbd, 0x00, 0x00, 0x00, 0x00]); // lea    rsi,[r15*4+0x0]
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x4c, 0x89, 0xd0]); // mov    rax,r10
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    Ok(())
}
