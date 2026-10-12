//! Bounded native text transformations. Helpers preserve RBX, RBP and R12–R15.
//! Matching builds one KMP prefix table and one bounded nonoverlapping-offset
//! array per operation. Scratch is released after the final immutable copy.
use super::*;

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    // Validated source RDI/RSI, needle R8/R9, EDX allocation family (0 Text, 1 List).
    // Return scratch RAX: qword count, u32 match offsets at 0x1000. Prefix table
    // begins at 0x402000. One 0x802000-byte mapping bounds both arrays even for
    // 1 MiB input and the 1,048,577 scalar boundaries of an empty needle.
    code.label("tt_matches")?;
    code.emit(&[0x53]); // push rbx
    code.emit(&[0x41, 0x54]); // push r12
    code.emit(&[0x41, 0x55]); // push r13
    code.emit(&[0x41, 0x56]); // push r14
    code.emit(&[0x41, 0x57]); // push r15
    code.emit(&[0x52]); // push rdx
    code.emit(&[0x49, 0x89, 0xfc]); // mov r12,rdi
    code.emit(&[0x49, 0x89, 0xf5]); // mov r13,rsi
    code.emit(&[0x4d, 0x89, 0xc6]); // mov r14,r8
    code.emit(&[0x4d, 0x89, 0xcf]); // mov r15,r9
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov eax,9
    code.emit(&[0x31, 0xff]); // xor edi,edi
    code.emit(&[0xbe, 0x00, 0x20, 0x80, 0x00]); // mov esi,0x802000
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov edx,3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov r8,-1
    code.emit(&[0x45, 0x31, 0xc9]); // xor r9d,r9d
    code.emit_call_label("recovery_temp_map");
    code.emit(&[0x48, 0x85, 0xc0]); // test rax,rax
    code.emit_jcc_label(&[0x0f, 0x89], "tt_matches_allocated");
    code.emit(&[0x48, 0x83, 0x3c, 0x24, 0x00]); // cmp qword ptr [rsp],0
    code.emit_jcc_label(&[0x0f, 0x85], "error_list_allocation");
    code.emit_jmp_label("error_text_allocation");
    code.label("tt_matches_allocated")?;
    code.emit(&[0x48, 0x83, 0xc4, 0x08]); // add rsp,8
    code.emit(&[0x48, 0x89, 0xc3]); // mov rbx,rax
    code.emit(&[0x45, 0x31, 0xd2]); // xor r10d,r10d
    code.emit(&[0x4d, 0x85, 0xff]); // test r15,r15
    code.emit_jcc_label(&[0x0f, 0x84], "tt_matches_empty");
    code.emit(&[0x4c, 0x8d, 0x83, 0x00, 0x20, 0x40, 0x00]); // lea r8,[rbx+0x402000]
    code.emit(&[0xb9, 0x01, 0x00, 0x00, 0x00]); // mov ecx,1
    code.emit(&[0x31, 0xd2]); // xor edx,edx
    code.label("tt_prefix_loop")?;
    code.emit(&[0x4c, 0x39, 0xf9]); // cmp rcx,r15
    code.emit_jcc_label(&[0x0f, 0x83], "tt_scan_start");
    code.emit(&[0x41, 0x8a, 0x04, 0x0e]); // mov al,[r14+rcx]
    code.label("tt_prefix_compare")?;
    code.emit(&[0x41, 0x3a, 0x04, 0x16]); // cmp al,[r14+rdx]
    code.emit_jcc_label(&[0x0f, 0x84], "tt_prefix_equal");
    code.emit(&[0x85, 0xd2]); // test edx,edx
    code.emit_jcc_label(&[0x0f, 0x84], "tt_prefix_store");
    code.emit(&[0x41, 0x8b, 0x54, 0x90, 0xfc]); // mov edx,[r8+rdx*4-4]
    code.emit_jmp_label("tt_prefix_compare");
    code.label("tt_prefix_equal")?;
    code.emit(&[0xff, 0xc2]); // inc edx
    code.label("tt_prefix_store")?;
    code.emit(&[0x41, 0x89, 0x14, 0x88]); // mov [r8+rcx*4],edx
    code.emit(&[0x48, 0xff, 0xc1]); // inc rcx
    code.emit_jmp_label("tt_prefix_loop");
    code.label("tt_scan_start")?;
    code.emit(&[0x31, 0xc9]); // xor ecx,ecx
    code.emit(&[0x31, 0xd2]); // xor edx,edx
    code.label("tt_scan_loop")?;
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp rcx,r13
    code.emit_jcc_label(&[0x0f, 0x83], "tt_matches_done");
    code.emit(&[0x41, 0x8a, 0x04, 0x0c]); // mov al,[r12+rcx]
    code.label("tt_scan_compare")?;
    code.emit(&[0x41, 0x3a, 0x04, 0x16]); // cmp al,[r14+rdx]
    code.emit_jcc_label(&[0x0f, 0x84], "tt_scan_equal");
    code.emit(&[0x85, 0xd2]); // test edx,edx
    code.emit_jcc_label(&[0x0f, 0x84], "tt_scan_next");
    code.emit(&[0x41, 0x8b, 0x54, 0x90, 0xfc]); // mov edx,[r8+rdx*4-4]
    code.emit_jmp_label("tt_scan_compare");
    code.label("tt_scan_equal")?;
    code.emit(&[0xff, 0xc2]); // inc edx
    code.label("tt_scan_next")?;
    code.emit(&[0x48, 0xff, 0xc1]); // inc rcx
    code.emit(&[0x4c, 0x39, 0xfa]); // cmp rdx,r15
    code.emit_jcc_label(&[0x0f, 0x85], "tt_scan_loop");
    code.emit(&[0x49, 0x89, 0xcb]); // mov r11,rcx
    code.emit(&[0x4d, 0x29, 0xfb]); // sub r11,r15
    code.emit(&[0x46, 0x89, 0x9c, 0x93, 0x00, 0x10, 0x00, 0x00]); // mov [rbx+r10*4+0x1000],r11d
    code.emit(&[0x49, 0xff, 0xc2]); // inc r10
    code.emit(&[0x31, 0xd2]); // xor edx,edx
    code.emit_jmp_label("tt_scan_loop");
    code.label("tt_matches_empty")?;
    code.emit(&[0x31, 0xc9]); // xor ecx,ecx
    code.label("tt_boundary_loop")?;
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp rcx,r13
    code.emit_jcc_label(&[0x0f, 0x84], "tt_boundary_store");
    code.emit(&[0x41, 0x8a, 0x04, 0x0c]); // mov al,[r12+rcx]
    code.emit(&[0x24, 0xc0]); // and al,0xc0
    code.emit(&[0x3c, 0x80]); // cmp al,0x80
    code.emit_jcc_label(&[0x0f, 0x84], "tt_boundary_next");
    code.label("tt_boundary_store")?;
    code.emit(&[0x42, 0x89, 0x8c, 0x93, 0x00, 0x10, 0x00, 0x00]); // mov [rbx+r10*4+0x1000],ecx
    code.emit(&[0x49, 0xff, 0xc2]); // inc r10
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp rcx,r13
    code.emit_jcc_label(&[0x0f, 0x84], "tt_matches_done");
    code.label("tt_boundary_next")?;
    code.emit(&[0x48, 0xff, 0xc1]); // inc rcx
    code.emit_jmp_label("tt_boundary_loop");
    code.label("tt_matches_done")?;
    code.emit(&[0x4c, 0x89, 0x13]); // mov [rbx],r10
    code.emit(&[0x48, 0x89, 0xd8]); // mov rax,rbx
    code.emit(&[0x41, 0x5f]); // pop r15
    code.emit(&[0x41, 0x5e]); // pop r14
    code.emit(&[0x41, 0x5d]); // pop r13
    code.emit(&[0x41, 0x5c]); // pop r12
    code.emit(&[0x5b]); // pop rbx
    code.emit(&[0xc3]); // ret
    // Unmap matcher scratch RDI without disturbing result RAX/RDX.
    code.label("tt_matches_release")?;
    code.emit(&[0x50]); // push rax
    code.emit(&[0x52]); // push rdx
    code.emit(&[0xb8, 0x0b, 0x00, 0x00, 0x00]); // mov eax,11
    code.emit(&[0xbe, 0x00, 0x20, 0x80, 0x00]); // mov esi,0x802000
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x5a]); // pop rdx
    code.emit(&[0x58]); // pop rax
    code.emit(&[0xc3]); // ret
    // Allocate the same scratch layout used by list_parse, for borrowed split slices.
    code.label("tt_split_scratch")?;
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov eax,9
    code.emit(&[0x31, 0xff]); // xor edi,edi
    code.emit(&[0xbe, 0x00, 0x10, 0x30, 0x00]); // mov esi,0x301000
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov edx,3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov r8,-1
    code.emit(&[0x45, 0x31, 0xc9]); // xor r9d,r9d
    code.emit_call_label("recovery_temp_map");
    code.emit(&[0x48, 0x85, 0xc0]); // test rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_list_allocation");
    code.emit(&[0xc3]); // ret
    // Validated nonempty separator R8/R9 and source RDI/RSI. Result RAX/RDX is
    // one immutable canonical List<Text>; descriptors borrow source until encoding.
    code.label("tt_split")?;
    code.emit(&[0x53]); // push rbx
    code.emit(&[0x41, 0x54]); // push r12
    code.emit(&[0x41, 0x55]); // push r13
    code.emit(&[0x41, 0x56]); // push r14
    code.emit(&[0x41, 0x57]); // push r15
    code.emit(&[0x49, 0x89, 0xfc]); // mov r12,rdi
    code.emit(&[0x49, 0x89, 0xf5]); // mov r13,rsi
    code.emit(&[0x4d, 0x89, 0xce]); // mov r14,r9
    code.emit(&[0xba, 0x01, 0x00, 0x00, 0x00]); // mov edx,1
    code.emit_call_label("tt_matches");
    code.emit(&[0x49, 0x89, 0xc7]); // mov r15,rax
    code.emit(&[0x49, 0x81, 0x3f, 0x00, 0x00, 0x01, 0x00]); // cmp qword ptr [r15],65536
    code.emit_jcc_label(&[0x0f, 0x83], "error_list_limit");
    code.emit_call_label("tt_split_scratch");
    code.emit(&[0x48, 0x89, 0xc3]); // mov rbx,rax
    code.emit(&[0x49, 0x8b, 0x07]); // mov rax,[r15]
    code.emit(&[0x48, 0xff, 0xc0]); // inc rax
    code.emit(&[0x48, 0x89, 0x03]); // mov [rbx],rax
    code.emit(&[0x48, 0xc7, 0x43, 0x08, 0x01, 0x00, 0x00, 0x00]); // mov qword ptr [rbx+8],1
    code.emit(&[0x45, 0x31, 0xd2]); // xor r10d,r10d
    code.emit(&[0x45, 0x31, 0xdb]); // xor r11d,r11d
    code.label("tt_split_descriptor")?;
    code.emit(&[0x4c, 0x89, 0xea]); // mov rdx,r13
    code.emit(&[0x4d, 0x3b, 0x17]); // cmp r10,[r15]
    code.emit_jcc_label(&[0x0f, 0x84], "tt_split_last");
    code.emit(&[0x43, 0x8b, 0x94, 0x97, 0x00, 0x10, 0x00, 0x00]); // mov edx,[r15+r10*4+0x1000]
    code.label("tt_split_last")?;
    code.emit(&[0x4c, 0x89, 0xd0]); // mov rax,r10
    code.emit(&[0x48, 0xc1, 0xe0, 0x04]); // shl rax,4
    code.emit(&[0x4b, 0x8d, 0x0c, 0x1c]); // lea rcx,[r12+r11]
    code.emit(&[0x48, 0x89, 0x8c, 0x03, 0x00, 0x10, 0x00, 0x00]); // mov [rbx+rax+0x1000],rcx
    code.emit(&[0x48, 0x89, 0xd1]); // mov rcx,rdx
    code.emit(&[0x4c, 0x29, 0xd9]); // sub rcx,r11
    code.emit(&[0x48, 0x89, 0x8c, 0x03, 0x08, 0x10, 0x00, 0x00]); // mov [rbx+rax+0x1008],rcx
    code.emit(&[0x4e, 0x8d, 0x1c, 0x32]); // lea r11,[rdx+r14]
    code.emit(&[0x49, 0xff, 0xc2]); // inc r10
    code.emit(&[0x4c, 0x3b, 0x13]); // cmp r10,[rbx]
    code.emit_jcc_label(&[0x0f, 0x82], "tt_split_descriptor");
    code.emit(&[0x48, 0x89, 0xdf]); // mov rdi,rbx
    code.emit_call_label("list_encode");
    code.emit(&[0x50]); // push rax
    code.emit_call_label("list_allocate");
    code.emit(&[0x5e]); // pop rsi
    code.emit(&[0x48, 0x89, 0xc7]); // mov rdi,rax
    code.emit(&[0x48, 0x89, 0xd1]); // mov rcx,rdx
    code.emit(&[0x50]); // push rax
    code.emit(&[0xf3, 0xa4]); // rep movsb
    code.emit(&[0x58]); // pop rax
    code.emit(&[0x48, 0x89, 0xdf]); // mov rdi,rbx
    code.emit_call_label("list_release");
    code.emit(&[0x4c, 0x89, 0xff]); // mov rdi,r15
    code.emit_call_label("tt_matches_release");
    code.emit(&[0x41, 0x5f]); // pop r15
    code.emit(&[0x41, 0x5e]); // pop r14
    code.emit(&[0x41, 0x5d]); // pop r13
    code.emit(&[0x41, 0x5c]); // pop r12
    code.emit(&[0x5b]); // pop rbx
    code.emit(&[0xc3]); // ret
    // Validated source RDI/RSI, needle R8/R9, replacement RDX/RCX.
    // Build offsets once, measure before allocating, then copy each final span once.
    code.label("tt_replace")?;
    code.emit(&[0x53]); // push rbx
    code.emit(&[0x41, 0x54]); // push r12
    code.emit(&[0x41, 0x55]); // push r13
    code.emit(&[0x41, 0x56]); // push r14
    code.emit(&[0x41, 0x57]); // push r15
    code.emit(&[0x48, 0x83, 0xec, 0x18]); // sub rsp,24
    code.emit(&[0x49, 0x89, 0xfc]); // mov r12,rdi
    code.emit(&[0x49, 0x89, 0xf5]); // mov r13,rsi
    code.emit(&[0x4d, 0x89, 0xce]); // mov r14,r9
    code.emit(&[0x49, 0x89, 0xd7]); // mov r15,rdx
    code.emit(&[0x48, 0x89, 0xcb]); // mov rbx,rcx
    code.emit(&[0x31, 0xd2]); // xor edx,edx
    code.emit_call_label("tt_matches");
    code.emit(&[0x48, 0x89, 0x04, 0x24]); // mov [rsp],rax
    code.emit(&[0x48, 0x8b, 0x08]); // mov rcx,[rax]
    code.emit(&[0x48, 0x89, 0xca]); // mov rdx,rcx
    code.emit(&[0x49, 0x0f, 0xaf, 0xd6]); // imul rdx,r14
    code.emit(&[0x4c, 0x89, 0xe8]); // mov rax,r13
    code.emit(&[0x48, 0x29, 0xd0]); // sub rax,rdx
    code.emit(&[0x48, 0x0f, 0xaf, 0xcb]); // imul rcx,rbx
    code.emit(&[0x48, 0x8d, 0x14, 0x08]); // lea rdx,[rax+rcx]
    code.emit_call_label("text_allocate");
    code.emit(&[0x48, 0x89, 0x44, 0x24, 0x08]); // mov [rsp+8],rax
    code.emit(&[0x48, 0x89, 0x54, 0x24, 0x10]); // mov [rsp+16],rdx
    code.emit(&[0x48, 0x89, 0xc7]); // mov rdi,rax
    code.emit(&[0x45, 0x31, 0xd2]); // xor r10d,r10d
    code.emit(&[0x45, 0x31, 0xdb]); // xor r11d,r11d
    code.label("tt_replace_copy")?;
    code.emit(&[0x4c, 0x8b, 0x04, 0x24]); // mov r8,[rsp]
    code.emit(&[0x4d, 0x3b, 0x10]); // cmp r10,[r8]
    code.emit_jcc_label(&[0x0f, 0x83], "tt_replace_tail");
    code.emit(&[0x43, 0x8b, 0x84, 0x90, 0x00, 0x10, 0x00, 0x00]); // mov eax,[r8+r10*4+0x1000]
    code.emit(&[0x4b, 0x8d, 0x34, 0x1c]); // lea rsi,[r12+r11]
    code.emit(&[0x48, 0x89, 0xc1]); // mov rcx,rax
    code.emit(&[0x4c, 0x29, 0xd9]); // sub rcx,r11
    code.emit(&[0xf3, 0xa4]); // rep movsb
    code.emit(&[0x4c, 0x89, 0xfe]); // mov rsi,r15
    code.emit(&[0x48, 0x89, 0xd9]); // mov rcx,rbx
    code.emit(&[0xf3, 0xa4]); // rep movsb
    code.emit(&[0x4e, 0x8d, 0x1c, 0x30]); // lea r11,[rax+r14]
    code.emit(&[0x49, 0xff, 0xc2]); // inc r10
    code.emit_jmp_label("tt_replace_copy");
    code.label("tt_replace_tail")?;
    code.emit(&[0x4b, 0x8d, 0x34, 0x1c]); // lea rsi,[r12+r11]
    code.emit(&[0x4c, 0x89, 0xe9]); // mov rcx,r13
    code.emit(&[0x4c, 0x29, 0xd9]); // sub rcx,r11
    code.emit(&[0xf3, 0xa4]); // rep movsb
    code.emit(&[0x48, 0x8b, 0x44, 0x24, 0x08]); // mov rax,[rsp+8]
    code.emit(&[0x48, 0x8b, 0x54, 0x24, 0x10]); // mov rdx,[rsp+16]
    code.emit(&[0x48, 0x8b, 0x3c, 0x24]); // mov rdi,[rsp]
    code.emit_call_label("tt_matches_release");
    code.emit(&[0x48, 0x83, 0xc4, 0x18]); // add rsp,24
    code.emit(&[0x41, 0x5f]); // pop r15
    code.emit(&[0x41, 0x5e]); // pop r14
    code.emit(&[0x41, 0x5d]); // pop r13
    code.emit(&[0x41, 0x5c]); // pop r12
    code.emit(&[0x5b]); // pop rbx
    code.emit(&[0xc3]); // ret
    // Parsed List<Text> scratch RDI, validated separator R8/R9. Validation of the
    // complete list precedes separator validation in the operation emitter.
    code.label("tt_join_list")?;
    code.emit(&[0x53]); // push rbx
    code.emit(&[0x41, 0x54]); // push r12
    code.emit(&[0x41, 0x55]); // push r13
    code.emit(&[0x41, 0x56]); // push r14
    code.emit(&[0x41, 0x57]); // push r15
    code.emit(&[0x49, 0x89, 0xfc]); // mov r12,rdi
    code.emit(&[0x4d, 0x89, 0xc5]); // mov r13,r8
    code.emit(&[0x4d, 0x89, 0xce]); // mov r14,r9
    code.emit(&[0x4d, 0x8b, 0x3c, 0x24]); // mov r15,[r12]
    code.emit(&[0x31, 0xd2]); // xor edx,edx
    code.emit(&[0x4d, 0x85, 0xff]); // test r15,r15
    code.emit_jcc_label(&[0x0f, 0x84], "tt_join_measure_done");
    code.emit(&[0x4c, 0x89, 0xfa]); // mov rdx,r15
    code.emit(&[0x48, 0xff, 0xca]); // dec rdx
    code.emit(&[0x49, 0x0f, 0xaf, 0xd6]); // imul rdx,r14
    code.emit(&[0x31, 0xdb]); // xor ebx,ebx
    code.label("tt_join_measure")?;
    code.emit(&[0x48, 0x89, 0xd9]); // mov rcx,rbx
    code.emit(&[0x48, 0xc1, 0xe1, 0x04]); // shl rcx,4
    code.emit(&[0x49, 0x03, 0x94, 0x0c, 0x08, 0x10, 0x00, 0x00]); // add rdx,[r12+rcx+0x1008]
    code.emit(&[0x48, 0xff, 0xc3]); // inc rbx
    code.emit(&[0x4c, 0x39, 0xfb]); // cmp rbx,r15
    code.emit_jcc_label(&[0x0f, 0x82], "tt_join_measure");
    code.label("tt_join_measure_done")?;
    code.emit_call_label("text_allocate");
    code.emit(&[0x50]); // push rax
    code.emit(&[0x52]); // push rdx
    code.emit(&[0x48, 0x89, 0xc7]); // mov rdi,rax
    code.emit(&[0x31, 0xdb]); // xor ebx,ebx
    code.label("tt_join_copy")?;
    code.emit(&[0x4c, 0x39, 0xfb]); // cmp rbx,r15
    code.emit_jcc_label(&[0x0f, 0x83], "tt_join_done");
    code.emit(&[0x48, 0x85, 0xdb]); // test rbx,rbx
    code.emit_jcc_label(&[0x0f, 0x84], "tt_join_item");
    code.emit(&[0x4c, 0x89, 0xee]); // mov rsi,r13
    code.emit(&[0x4c, 0x89, 0xf1]); // mov rcx,r14
    code.emit(&[0xf3, 0xa4]); // rep movsb
    code.label("tt_join_item")?;
    code.emit(&[0x48, 0x89, 0xd9]); // mov rcx,rbx
    code.emit(&[0x48, 0xc1, 0xe1, 0x04]); // shl rcx,4
    code.emit(&[0x49, 0x8b, 0xb4, 0x0c, 0x00, 0x10, 0x00, 0x00]); // mov rsi,[r12+rcx+0x1000]
    code.emit(&[0x49, 0x8b, 0x8c, 0x0c, 0x08, 0x10, 0x00, 0x00]); // mov rcx,[r12+rcx+0x1008]
    code.emit(&[0xf3, 0xa4]); // rep movsb
    code.emit(&[0x48, 0xff, 0xc3]); // inc rbx
    code.emit_jmp_label("tt_join_copy");
    code.label("tt_join_done")?;
    code.emit(&[0x5a]); // pop rdx
    code.emit(&[0x58]); // pop rax
    code.emit(&[0x4c, 0x89, 0xe7]); // mov rdi,r12
    code.emit_call_label("list_release");
    code.emit(&[0x41, 0x5f]); // pop r15
    code.emit(&[0x41, 0x5e]); // pop r14
    code.emit(&[0x41, 0x5d]); // pop r13
    code.emit(&[0x41, 0x5c]); // pop r12
    code.emit(&[0x5b]); // pop rbx
    code.emit(&[0xc3]); // ret
    // Validated Text RDI/RSI. Decode scalars, retain the first through last nonspace
    // span, and charge/copy exactly once even for unchanged or empty text.
    code.label("tt_trim")?;
    code.emit(&[0x53]); // push rbx
    code.emit(&[0x41, 0x54]); // push r12
    code.emit(&[0x41, 0x55]); // push r13
    code.emit(&[0x41, 0x56]); // push r14
    code.emit(&[0x41, 0x57]); // push r15
    code.emit(&[0x49, 0x89, 0xfc]); // mov r12,rdi
    code.emit(&[0x4c, 0x8d, 0x2c, 0x37]); // lea r13,[rdi+rsi]
    code.emit(&[0x4d, 0x89, 0xee]); // mov r14,r13
    code.emit(&[0x4d, 0x89, 0xef]); // mov r15,r13
    code.label("tt_trim_scan")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp r12,r13
    code.emit_jcc_label(&[0x0f, 0x83], "tt_trim_copy");
    code.emit(&[0x4c, 0x89, 0xe3]); // mov rbx,r12
    code.emit(&[0x41, 0x0f, 0xb6, 0x04, 0x24]); // movzx eax,byte ptr [r12]
    code.emit(&[0x49, 0xff, 0xc4]); // inc r12
    code.emit(&[0x3d, 0x80, 0x00, 0x00, 0x00]); // cmp eax,0x80
    code.emit_jcc_label(&[0x0f, 0x82], "tt_trim_classify");
    code.emit(&[0xb9, 0x01, 0x00, 0x00, 0x00]); // mov ecx,1
    code.emit(&[0x83, 0xe0, 0x1f]); // and eax,0x1f
    code.emit(&[0x0f, 0xb6, 0x13]); // movzx edx,byte ptr [rbx]
    code.emit(&[0x81, 0xfa, 0xe0, 0x00, 0x00, 0x00]); // cmp edx,0xe0
    code.emit_jcc_label(&[0x0f, 0x82], "tt_trim_decode");
    code.emit(&[0xff, 0xc1]); // inc ecx
    code.emit(&[0x83, 0xe0, 0x0f]); // and eax,0x0f
    code.emit(&[0x81, 0xfa, 0xf0, 0x00, 0x00, 0x00]); // cmp edx,0xf0
    code.emit_jcc_label(&[0x0f, 0x82], "tt_trim_decode");
    code.emit(&[0xff, 0xc1]); // inc ecx
    code.emit(&[0x83, 0xe0, 0x07]); // and eax,7
    code.label("tt_trim_decode")?;
    code.emit(&[0xc1, 0xe0, 0x06]); // shl eax,6
    code.emit(&[0x41, 0x0f, 0xb6, 0x14, 0x24]); // movzx edx,byte ptr [r12]
    code.emit(&[0x83, 0xe2, 0x3f]); // and edx,0x3f
    code.emit(&[0x09, 0xd0]); // or eax,edx
    code.emit(&[0x49, 0xff, 0xc4]); // inc r12
    code.emit(&[0xff, 0xc9]); // dec ecx
    code.emit_jcc_label(&[0x0f, 0x85], "tt_trim_decode");
    code.label("tt_trim_classify")?;
    code.emit(&[0x83, 0xf8, 0x09]); // cmp eax,9
    code.emit_jcc_label(&[0x0f, 0x82], "tt_trim_nonspace");
    code.emit(&[0x83, 0xf8, 0x0d]); // cmp eax,13
    code.emit_jcc_label(&[0x0f, 0x86], "tt_trim_scan");
    code.emit(&[0x83, 0xf8, 0x1c]); // cmp eax,0x1c
    code.emit_jcc_label(&[0x0f, 0x82], "tt_trim_nonspace");
    code.emit(&[0x83, 0xf8, 0x20]); // cmp eax,0x20
    code.emit_jcc_label(&[0x0f, 0x86], "tt_trim_scan");
    code.emit(&[0x3d, 0x85, 0x00, 0x00, 0x00]); // cmp eax,0x85
    code.emit_jcc_label(&[0x0f, 0x84], "tt_trim_scan");
    code.emit(&[0x3d, 0xa0, 0x00, 0x00, 0x00]); // cmp eax,0xa0
    code.emit_jcc_label(&[0x0f, 0x84], "tt_trim_scan");
    code.emit(&[0x3d, 0x80, 0x16, 0x00, 0x00]); // cmp eax,0x1680
    code.emit_jcc_label(&[0x0f, 0x84], "tt_trim_scan");
    code.emit(&[0x3d, 0x00, 0x20, 0x00, 0x00]); // cmp eax,0x2000
    code.emit_jcc_label(&[0x0f, 0x82], "tt_trim_nonspace");
    code.emit(&[0x3d, 0x0a, 0x20, 0x00, 0x00]); // cmp eax,0x200a
    code.emit_jcc_label(&[0x0f, 0x86], "tt_trim_scan");
    code.emit(&[0x3d, 0x28, 0x20, 0x00, 0x00]); // cmp eax,0x2028
    code.emit_jcc_label(&[0x0f, 0x84], "tt_trim_scan");
    code.emit(&[0x3d, 0x29, 0x20, 0x00, 0x00]); // cmp eax,0x2029
    code.emit_jcc_label(&[0x0f, 0x84], "tt_trim_scan");
    code.emit(&[0x3d, 0x2f, 0x20, 0x00, 0x00]); // cmp eax,0x202f
    code.emit_jcc_label(&[0x0f, 0x84], "tt_trim_scan");
    code.emit(&[0x3d, 0x5f, 0x20, 0x00, 0x00]); // cmp eax,0x205f
    code.emit_jcc_label(&[0x0f, 0x84], "tt_trim_scan");
    code.emit(&[0x3d, 0x00, 0x30, 0x00, 0x00]); // cmp eax,0x3000
    code.emit_jcc_label(&[0x0f, 0x84], "tt_trim_scan");
    code.label("tt_trim_nonspace")?;
    code.emit(&[0x4d, 0x39, 0xee]); // cmp r14,r13
    code.emit_jcc_label(&[0x0f, 0x85], "tt_trim_end");
    code.emit(&[0x49, 0x89, 0xde]); // mov r14,rbx
    code.label("tt_trim_end")?;
    code.emit(&[0x4d, 0x89, 0xe7]); // mov r15,r12
    code.emit_jmp_label("tt_trim_scan");
    code.label("tt_trim_copy")?;
    code.emit(&[0x4c, 0x89, 0xfa]); // mov rdx,r15
    code.emit(&[0x4c, 0x29, 0xf2]); // sub rdx,r14
    code.emit_call_label("text_allocate");
    code.emit(&[0x4c, 0x89, 0xf6]); // mov rsi,r14
    code.emit(&[0x48, 0x89, 0xc7]); // mov rdi,rax
    code.emit(&[0x48, 0x89, 0xd1]); // mov rcx,rdx
    code.emit(&[0x50]); // push rax
    code.emit(&[0xf3, 0xa4]); // rep movsb
    code.emit(&[0x58]); // pop rax
    code.emit(&[0x41, 0x5f]); // pop r15
    code.emit(&[0x41, 0x5e]); // pop r14
    code.emit(&[0x41, 0x5d]); // pop r13
    code.emit(&[0x41, 0x5c]); // pop r12
    code.emit(&[0x5b]); // pop rbx
    code.emit(&[0xc3]); // ret
    Ok(())
}
