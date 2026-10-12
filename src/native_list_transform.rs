//! Native typed-list transformations over validated 16-byte descriptors.
//! Stable bottom-up mergesort reuses the canonical buffer as bounded scratch.
//! Helpers preserve RBX, RBP and R12–R15 and charge only the final owned copy.
use super::*;

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    // RDI parsed scratch -> one canonical owned result in RAX/RDX.
    // Parsing and transformations never charge either arena; only this copy does.
    code.label("list_transform_finish")?;
    code.emit_call_label("list_encode");
    code.emit(&[0x50]); // push rax
    code.emit_call_label("list_allocate");
    code.emit(&[0x5e]); // pop rsi
    code.emit(&[0x48, 0x89, 0xc7]); // mov rdi, rax
    code.emit(&[0x48, 0x89, 0xd1]); // mov rcx, rdx
    code.emit(&[0x50]); // push rax
    code.emit(&[0xf3, 0xa4]); // rep movsb
    code.emit(&[0x58]); // pop rax
    code.emit(&[0xc3]); // ret
    // RDI/RSI are completely validated inputs. Keep both mappings alive while
    // encoding: appended Text descriptors still point into the second mapping.
    code.label("list_transform_concat")?;
    code.emit(&[0x57]); // push rdi
    code.emit(&[0x48, 0x8b, 0x07]); // mov rax, [rdi]
    code.emit(&[0x48, 0x8b, 0x0e]); // mov rcx, [rsi]
    code.emit(&[0x48, 0x8d, 0x14, 0x08]); // lea rdx, [rax+rcx]
    code.emit(&[0x48, 0x81, 0xfa, 0x00, 0x00, 0x01, 0x00]); // cmp rdx, 65536
    code.emit_jcc_label(&[0x0f, 0x87], "error_list_limit");
    code.emit(&[0x48, 0x89, 0x17]); // mov [rdi], rdx
    code.emit(&[0x48, 0xc1, 0xe0, 0x04]); // shl rax, 4
    code.emit(&[0x48, 0x8d, 0xbc, 0x07, 0x00, 0x10, 0x00, 0x00]); // lea rdi, [rdi+rax+0x1000]
    code.emit(&[0x48, 0x81, 0xc6, 0x00, 0x10, 0x00, 0x00]); // add rsi, 0x1000
    code.emit(&[0x48, 0xd1, 0xe1]); // shl rcx, 1
    code.emit(&[0xf3, 0x48, 0xa5]); // rep movsq
    code.emit(&[0x5f]); // pop rdi
    code.emit_jmp_label("list_transform_finish");
    // RDI scratch, RSI signed start, RDX signed end. Negative addition cannot
    // overflow because the count is nonnegative and at most 65,536.
    code.label("list_transform_slice")?;
    code.emit(&[0x4c, 0x8b, 0x07]); // mov r8, [rdi]
    code.emit(&[0x31, 0xc0]); // xor eax, eax
    code.emit(&[0x48, 0x85, 0xf6]); // test rsi, rsi
    code.emit_jcc_label(&[0x0f, 0x89], "list_transform_slice_start_positive");
    code.emit(&[0x4c, 0x01, 0xc6]); // add rsi, r8
    code.emit(&[0x48, 0x0f, 0x48, 0xf0]); // cmovs rsi, rax
    code.label("list_transform_slice_start_positive")?;
    code.emit(&[0x4c, 0x39, 0xc6]); // cmp rsi, r8
    code.emit(&[0x49, 0x0f, 0x4f, 0xf0]); // cmovg rsi, r8
    code.emit(&[0x48, 0x85, 0xd2]); // test rdx, rdx
    code.emit_jcc_label(&[0x0f, 0x89], "list_transform_slice_end_positive");
    code.emit(&[0x4c, 0x01, 0xc2]); // add rdx, r8
    code.emit(&[0x48, 0x0f, 0x48, 0xd0]); // cmovs rdx, rax
    code.label("list_transform_slice_end_positive")?;
    code.emit(&[0x4c, 0x39, 0xc2]); // cmp rdx, r8
    code.emit(&[0x49, 0x0f, 0x4f, 0xd0]); // cmovg rdx, r8
    code.emit(&[0x48, 0x39, 0xf2]); // cmp rdx, rsi
    code.emit(&[0x48, 0x0f, 0x4c, 0xd6]); // cmovl rdx, rsi
    code.emit(&[0x48, 0x29, 0xf2]); // sub rdx, rsi
    code.emit(&[0x48, 0x89, 0x17]); // mov [rdi], rdx
    code.emit(&[0x57]); // push rdi
    code.emit(&[0x48, 0x89, 0xd1]); // mov rcx, rdx
    code.emit(&[0x48, 0xd1, 0xe1]); // shl rcx, 1
    code.emit(&[0x48, 0xc1, 0xe6, 0x04]); // shl rsi, 4
    code.emit(&[0x48, 0x8d, 0xb4, 0x37, 0x00, 0x10, 0x00, 0x00]); // lea rsi, [rdi+rsi+0x1000]
    code.emit(&[0x48, 0x81, 0xc7, 0x00, 0x10, 0x00, 0x00]); // add rdi, 0x1000
    code.emit(&[0xf3, 0x48, 0xa5]); // rep movsq
    code.emit(&[0x5f]); // pop rdi
    code.emit_jmp_label("list_transform_finish");
    // Reverse complete 16-byte descriptors, leaving decoded Text storage intact.
    code.label("list_transform_reverse")?;
    code.emit(&[0x48, 0x8b, 0x0f]); // mov rcx, [rdi]
    code.emit(&[0x48, 0x83, 0xf9, 0x01]); // cmp rcx, 1
    code.emit_jcc_label(&[0x0f, 0x86], "list_transform_finish");
    code.emit(&[0x4c, 0x8d, 0x87, 0x00, 0x10, 0x00, 0x00]); // lea r8, [rdi+0x1000]
    code.emit(&[0x48, 0xc1, 0xe1, 0x04]); // shl rcx, 4
    code.emit(&[0x4d, 0x8d, 0x4c, 0x08, 0xf0]); // lea r9, [r8+rcx-16]
    code.label("list_transform_reverse_loop")?;
    code.emit(&[0x49, 0x8b, 0x00]); // mov rax, [r8]
    code.emit(&[0x49, 0x8b, 0x50, 0x08]); // mov rdx, [r8+8]
    code.emit(&[0x4d, 0x8b, 0x11]); // mov r10, [r9]
    code.emit(&[0x4d, 0x8b, 0x59, 0x08]); // mov r11, [r9+8]
    code.emit(&[0x4d, 0x89, 0x10]); // mov [r8], r10
    code.emit(&[0x4d, 0x89, 0x58, 0x08]); // mov [r8+8], r11
    code.emit(&[0x49, 0x89, 0x01]); // mov [r9], rax
    code.emit(&[0x49, 0x89, 0x51, 0x08]); // mov [r9+8], rdx
    code.emit(&[0x49, 0x83, 0xc0, 0x10]); // add r8, 16
    code.emit(&[0x49, 0x83, 0xe9, 0x10]); // sub r9, 16
    code.emit(&[0x4d, 0x39, 0xc8]); // cmp r8, r9
    code.emit_jcc_label(&[0x0f, 0x82], "list_transform_reverse_loop");
    code.emit_jmp_label("list_transform_finish");
    // Stable bottom-up mergesort: RDI scratch, ESI descending. The canonical
    // output region is an auxiliary descriptor array until sorting finishes.
    // The decoded Text region lies between both arrays and remains untouched.
    code.label("list_transform_sort")?;
    code.emit(&[0x53]); // push rbx
    code.emit(&[0x41, 0x54]); // push r12
    code.emit(&[0x41, 0x55]); // push r13
    code.emit(&[0x41, 0x56]); // push r14
    code.emit(&[0x41, 0x57]); // push r15
    code.emit(&[0x48, 0x83, 0xec, 0x30]); // sub rsp, 48
    code.emit(&[0x48, 0x89, 0x34, 0x24]); // mov [rsp], rsi
    code.emit(&[0x48, 0x89, 0xfb]); // mov rbx, rdi
    code.emit(&[0x4c, 0x8b, 0x37]); // mov r14, [rdi]
    code.emit(&[0x4c, 0x8d, 0xa7, 0x00, 0x10, 0x00, 0x00]); // lea r12, [rdi+0x1000]
    code.emit(&[0x4c, 0x8d, 0xaf, 0x00, 0x10, 0x20, 0x00]); // lea r13, [rdi+0x201000]
    code.emit(&[0x41, 0xbf, 0x01, 0x00, 0x00, 0x00]); // mov r15d, 1
    code.label("list_transform_sort_pass")?;
    code.emit(&[0x4d, 0x39, 0xf7]); // cmp r15, r14
    code.emit_jcc_label(&[0x0f, 0x83], "list_transform_sort_done");
    code.emit(&[0x31, 0xc0]); // xor eax, eax
    code.label("list_transform_sort_run")?;
    code.emit(&[0x4a, 0x8d, 0x14, 0x38]); // lea rdx, [rax+r15]
    code.emit(&[0x4c, 0x39, 0xf2]); // cmp rdx, r14
    code.emit(&[0x49, 0x0f, 0x47, 0xd6]); // cmova rdx, r14
    code.emit(&[0x4a, 0x8d, 0x0c, 0x3a]); // lea rcx, [rdx+r15]
    code.emit(&[0x4c, 0x39, 0xf1]); // cmp rcx, r14
    code.emit(&[0x49, 0x0f, 0x47, 0xce]); // cmova rcx, r14
    code.emit(&[0x48, 0x89, 0x4c, 0x24, 0x20]); // mov [rsp+32], rcx
    code.emit(&[0x48, 0xc1, 0xe0, 0x04]); // shl rax, 4
    code.emit(&[0x48, 0xc1, 0xe2, 0x04]); // shl rdx, 4
    code.emit(&[0x48, 0xc1, 0xe1, 0x04]); // shl rcx, 4
    code.emit(&[0x49, 0x8d, 0x3c, 0x04]); // lea rdi, [r12+rax]
    code.emit(&[0x49, 0x8d, 0x34, 0x14]); // lea rsi, [r12+rdx]
    code.emit(&[0x48, 0x89, 0x74, 0x24, 0x08]); // mov [rsp+8], rsi
    code.emit(&[0x49, 0x8d, 0x14, 0x0c]); // lea rdx, [r12+rcx]
    code.emit(&[0x48, 0x89, 0x54, 0x24, 0x10]); // mov [rsp+16], rdx
    code.emit(&[0x49, 0x8d, 0x54, 0x05, 0x00]); // lea rdx, [r13+rax]
    code.emit(&[0x48, 0x89, 0x54, 0x24, 0x18]); // mov [rsp+24], rdx
    code.label("list_transform_sort_merge")?;
    code.emit(&[0x48, 0x3b, 0x7c, 0x24, 0x08]); // cmp rdi, [rsp+8]
    code.emit_jcc_label(&[0x0f, 0x83], "list_transform_sort_left_done");
    code.emit(&[0x48, 0x3b, 0x74, 0x24, 0x10]); // cmp rsi, [rsp+16]
    code.emit_jcc_label(&[0x0f, 0x83], "list_transform_sort_copy_left");
    code.emit(&[0x8b, 0x53, 0x08]); // mov edx, [rbx+8]
    code.emit(&[0x8b, 0x0c, 0x24]); // mov ecx, [rsp]
    code.emit_call_label("list_transform_compare");
    code.emit(&[0x85, 0xc0]); // test eax, eax
    code.emit_jcc_label(&[0x0f, 0x8e], "list_transform_sort_copy_left");
    code.label("list_transform_sort_copy_right")?;
    code.emit(&[0x48, 0x8b, 0x06]); // mov rax, [rsi]
    code.emit(&[0x48, 0x8b, 0x56, 0x08]); // mov rdx, [rsi+8]
    code.emit(&[0x48, 0x83, 0xc6, 0x10]); // add rsi, 16
    code.emit_jmp_label("list_transform_sort_write");
    code.label("list_transform_sort_left_done")?;
    code.emit(&[0x48, 0x3b, 0x74, 0x24, 0x10]); // cmp rsi, [rsp+16]
    code.emit_jcc_label(&[0x0f, 0x82], "list_transform_sort_copy_right");
    code.emit(&[0x48, 0x8b, 0x44, 0x24, 0x20]); // mov rax, [rsp+32]
    code.emit(&[0x4c, 0x39, 0xf0]); // cmp rax, r14
    code.emit_jcc_label(&[0x0f, 0x82], "list_transform_sort_run");
    code.emit(&[0x4d, 0x87, 0xec]); // xchg r12, r13
    code.emit(&[0x49, 0xd1, 0xe7]); // shl r15, 1
    code.emit_jmp_label("list_transform_sort_pass");
    code.label("list_transform_sort_copy_left")?;
    code.emit(&[0x48, 0x8b, 0x07]); // mov rax, [rdi]
    code.emit(&[0x48, 0x8b, 0x57, 0x08]); // mov rdx, [rdi+8]
    code.emit(&[0x48, 0x83, 0xc7, 0x10]); // add rdi, 16
    code.label("list_transform_sort_write")?;
    code.emit(&[0x48, 0x8b, 0x4c, 0x24, 0x18]); // mov rcx, [rsp+24]
    code.emit(&[0x48, 0x89, 0x01]); // mov [rcx], rax
    code.emit(&[0x48, 0x89, 0x51, 0x08]); // mov [rcx+8], rdx
    code.emit(&[0x48, 0x83, 0xc1, 0x10]); // add rcx, 16
    code.emit(&[0x48, 0x89, 0x4c, 0x24, 0x18]); // mov [rsp+24], rcx
    code.emit_jmp_label("list_transform_sort_merge");
    code.label("list_transform_sort_done")?;
    code.emit(&[0x48, 0x8d, 0xbb, 0x00, 0x10, 0x00, 0x00]); // lea rdi, [rbx+0x1000]
    code.emit(&[0x49, 0x39, 0xfc]); // cmp r12, rdi
    code.emit_jcc_label(&[0x0f, 0x84], "list_transform_sort_encode");
    code.emit(&[0x4c, 0x89, 0xe6]); // mov rsi, r12
    code.emit(&[0x4b, 0x8d, 0x0c, 0x36]); // lea rcx, [r14+r14]
    code.emit(&[0xf3, 0x48, 0xa5]); // rep movsq
    code.label("list_transform_sort_encode")?;
    code.emit(&[0x48, 0x89, 0xdf]); // mov rdi, rbx
    code.emit_call_label("list_transform_finish");
    code.emit(&[0x48, 0x83, 0xc4, 0x30]); // add rsp, 48
    code.emit(&[0x41, 0x5f]); // pop r15
    code.emit(&[0x41, 0x5e]); // pop r14
    code.emit(&[0x41, 0x5d]); // pop r13
    code.emit(&[0x41, 0x5c]); // pop r12
    code.emit(&[0x5b]); // pop rbx
    code.emit(&[0xc3]); // ret
    // RDI/RSI descriptor pointers remain unchanged; EDX kind, ECX direction.
    // Returns signed -1/0/+1. Merge chooses the left descriptor on equality.
    code.label("list_transform_compare")?;
    code.emit(&[0x83, 0xfa, 0x01]); // cmp edx, 1
    code.emit_jcc_label(&[0x0f, 0x84], "list_transform_compare_text");
    code.emit(&[0x83, 0xfa, 0x02]); // cmp edx, 2
    code.emit_jcc_label(&[0x0f, 0x84], "list_transform_compare_float");
    code.emit(&[0x48, 0x8b, 0x07]); // mov rax, [rdi]
    code.emit(&[0x48, 0x3b, 0x06]); // cmp rax, [rsi]
    code.emit_jcc_label(&[0x0f, 0x8c], "list_transform_compare_less");
    code.emit_jcc_label(&[0x0f, 0x8f], "list_transform_compare_greater");
    code.emit_jmp_label("list_transform_compare_equal");
    code.label("list_transform_compare_text")?;
    code.emit(&[0x4c, 0x8b, 0x47, 0x08]); // mov r8, [rdi+8]
    code.emit(&[0x4c, 0x8b, 0x4e, 0x08]); // mov r9, [rsi+8]
    code.emit(&[0x4c, 0x8b, 0x17]); // mov r10, [rdi]
    code.emit(&[0x4c, 0x8b, 0x1e]); // mov r11, [rsi]
    code.label("list_transform_compare_text_byte")?;
    code.emit(&[0x4d, 0x85, 0xc0]); // test r8, r8
    code.emit_jcc_label(&[0x0f, 0x84], "list_transform_compare_text_prefix");
    code.emit(&[0x4d, 0x85, 0xc9]); // test r9, r9
    code.emit_jcc_label(&[0x0f, 0x84], "list_transform_compare_greater");
    code.emit(&[0x41, 0x8a, 0x02]); // mov al, [r10]
    code.emit(&[0x41, 0x3a, 0x03]); // cmp al, [r11]
    code.emit_jcc_label(&[0x0f, 0x82], "list_transform_compare_less");
    code.emit_jcc_label(&[0x0f, 0x87], "list_transform_compare_greater");
    code.emit(&[0x49, 0xff, 0xc2]); // inc r10
    code.emit(&[0x49, 0xff, 0xc3]); // inc r11
    code.emit(&[0x49, 0xff, 0xc8]); // dec r8
    code.emit(&[0x49, 0xff, 0xc9]); // dec r9
    code.emit_jmp_label("list_transform_compare_text_byte");
    code.label("list_transform_compare_text_prefix")?;
    code.emit(&[0x4d, 0x85, 0xc9]); // test r9, r9
    code.emit_jcc_label(&[0x0f, 0x85], "list_transform_compare_less");
    code.emit_jmp_label("list_transform_compare_equal");
    // Unsigned monotonic bit keys give exact IEEE numeric ordering without SSE
    // or Float runtime calls. Treat both zero signs as equal before creating keys.
    // NaNs return immediately so descending never moves them before numbers.
    code.label("list_transform_compare_float")?;
    code.emit(&[0x48, 0x8b, 0x07]); // mov rax, [rdi]
    code.emit(&[0x4c, 0x8b, 0x06]); // mov r8, [rsi]
    code.emit(&[0x49, 0x89, 0xc1]); // mov r9, rax
    code.emit(&[0x4d, 0x89, 0xc2]); // mov r10, r8
    code.emit(&[0x49, 0xd1, 0xe1]); // shl r9, 1
    code.emit(&[0x49, 0xd1, 0xe2]); // shl r10, 1
    code.emit(&[0x49, 0xbb, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0xff]); // mov r11, 0xffe0000000000000
    code.emit(&[0x4d, 0x39, 0xd9]); // cmp r9, r11
    code.emit_jcc_label(&[0x0f, 0x87], "list_transform_compare_left_nan");
    code.emit(&[0x4d, 0x39, 0xda]); // cmp r10, r11
    code.emit_jcc_label(&[0x0f, 0x87], "list_transform_compare_right_nan");
    code.emit(&[0x4d, 0x09, 0xd1]); // or r9, r10
    code.emit_jcc_label(&[0x0f, 0x84], "list_transform_compare_equal");
    code.emit(&[0x49, 0xbb, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80]); // mov r11, 0x8000000000000000
    code.emit(&[0x48, 0x85, 0xc0]); // test rax, rax
    code.emit_jcc_label(&[0x0f, 0x88], "list_transform_compare_left_negative");
    code.emit(&[0x4c, 0x31, 0xd8]); // xor rax, r11
    code.emit_jmp_label("list_transform_compare_right_key");
    code.label("list_transform_compare_left_negative")?;
    code.emit(&[0x48, 0xf7, 0xd0]); // not rax
    code.label("list_transform_compare_right_key")?;
    code.emit(&[0x4d, 0x85, 0xc0]); // test r8, r8
    code.emit_jcc_label(&[0x0f, 0x88], "list_transform_compare_right_negative");
    code.emit(&[0x4d, 0x31, 0xd8]); // xor r8, r11
    code.emit_jmp_label("list_transform_compare_keys");
    code.label("list_transform_compare_right_negative")?;
    code.emit(&[0x49, 0xf7, 0xd0]); // not r8
    code.label("list_transform_compare_keys")?;
    code.emit(&[0x4c, 0x39, 0xc0]); // cmp rax, r8
    code.emit_jcc_label(&[0x0f, 0x82], "list_transform_compare_less");
    code.emit_jcc_label(&[0x0f, 0x87], "list_transform_compare_greater");
    code.emit_jmp_label("list_transform_compare_equal");
    code.label("list_transform_compare_left_nan")?;
    code.emit(&[0x4d, 0x39, 0xda]); // cmp r10, r11
    code.emit_jcc_label(&[0x0f, 0x87], "list_transform_compare_equal");
    code.emit(&[0xb8, 0x01, 0x00, 0x00, 0x00]); // mov eax, 1
    code.emit(&[0xc3]); // ret
    code.label("list_transform_compare_right_nan")?;
    code.emit(&[0xb8, 0xff, 0xff, 0xff, 0xff]); // mov eax, -1
    code.emit(&[0xc3]); // ret
    code.label("list_transform_compare_less")?;
    code.emit(&[0xb8, 0xff, 0xff, 0xff, 0xff]); // mov eax, -1
    code.emit_jmp_label("list_transform_compare_direction");
    code.label("list_transform_compare_greater")?;
    code.emit(&[0xb8, 0x01, 0x00, 0x00, 0x00]); // mov eax, 1
    code.label("list_transform_compare_direction")?;
    code.emit(&[0x85, 0xc9]); // test ecx, ecx
    code.emit_jcc_label(&[0x0f, 0x84], "list_transform_compare_return");
    code.emit(&[0xf7, 0xd8]); // neg eax
    code.label("list_transform_compare_return")?;
    code.emit(&[0xc3]); // ret
    code.label("list_transform_compare_equal")?;
    code.emit(&[0x31, 0xc0]); // xor eax, eax
    code.emit(&[0xc3]); // ret
    Ok(())
}
