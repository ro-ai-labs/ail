//! Native structured recovery and tracked temporary mappings.
//!
//! RBP always addresses the root execution context: Text arena/counter at 0/8,
//! List arena/counter at 16/24, handler head at 32, temporary mapping head at 40,
//! live fuel-cell pointer at 48, output-scope head at 56, root fuel at 64, and
//! immutable process-wide standard-stream mode/grant at 72.
//! Function frames add a 24-byte footer: live fuel, previous fuel-cell pointer,
//! and the handler head on entry. Persistent arena pointers/counters never unwind.
//!
//! Each 88-byte handler record stores previous/mask/target/destination at 0..24,
//! owner RSP/RBX/R12/R13/R15 at 32..64, live fuel-cell pointer at 72, and output
//! checkpoint at 80. Throws may occur with helper scratch in these registers;
//! the record and fuel cell are authoritative. No temporary mappings survive a
//! completed statement or call entry, so recovery can release the whole registry.
use super::*;

pub(super) const ROOT_BYTES: usize = 80;
pub(super) const FUNCTION_CONTEXT: usize = 24;
pub(super) const HANDLER_BYTES: usize = 88;

pub(super) fn emit_raise(
    code: &mut X64Code,
    instruction: &AilBytecodeInstruction,
) -> Result<(), String> {
    recovery::verify_raise(instruction)?;
    let category = &instruction.operands["category"];
    // The existing labels select recovery or their fatal diagnostic. Do not
    // return through the current function or publish a call result on this path.
    code.emit_jmp_label(&format!(
        "error_{}",
        category.to_ascii_lowercase().replace('-', "_")
    ));
    Ok(())
}

fn emit_storage_helpers(code: &mut X64Code) -> Result<(), String> {
    // Temporary payloads follow a hidden 4 KiB header containing previous,
    // next, and total mapping length. Persistent arenas do not use this helper.
    // Match mmap's register contract: only RAX, RCX, R11 may change.
    code.label("recovery_temp_map")?;
    code.emit(&[0x57]); // push rdi
    code.emit(&[0x56]); // push rsi
    code.emit(&[0x52]); // push rdx
    code.emit(&[0x41, 0x50]); // push r8
    code.emit(&[0x41, 0x51]); // push r9
    code.emit(&[0x41, 0x52]); // push r10
    code.emit(&[0x48, 0x81, 0xc6, 0x00, 0x10, 0x00, 0x00]); // add rsi, 4096
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov eax, 9
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x85, 0xc0]); // test rax, rax
    code.emit_jcc_label(&[0x0f, 0x88], "recovery_temp_map_done");
    code.emit(&[0x48, 0xc7, 0x00, 0x00, 0x00, 0x00, 0x00]); // mov qword ptr [rax], 0
    code.emit(&[0x4c, 0x8b, 0x45, 0x28]); // mov r8, [rbp+40]
    code.emit(&[0x4c, 0x89, 0x40, 0x08]); // mov [rax+8], r8
    code.emit(&[0x48, 0x89, 0x70, 0x10]); // mov [rax+16], rsi
    code.emit(&[0x4d, 0x85, 0xc0]); // test r8, r8
    code.emit_jcc_label(&[0x0f, 0x84], "recovery_temp_map_link");
    code.emit(&[0x49, 0x89, 0x00]); // mov [r8], rax
    code.label("recovery_temp_map_link")?;
    code.emit(&[0x48, 0x89, 0x45, 0x28]); // mov [rbp+40], rax
    code.emit(&[0x48, 0x05, 0x00, 0x10, 0x00, 0x00]); // add rax, 4096
    code.label("recovery_temp_map_done")?;
    code.emit(&[0x41, 0x5a]); // pop r10
    code.emit(&[0x41, 0x59]); // pop r9
    code.emit(&[0x41, 0x58]); // pop r8
    code.emit(&[0x5a]); // pop rdx
    code.emit(&[0x5e]); // pop rsi
    code.emit(&[0x5f]); // pop rdi
    code.emit(&[0xc3]); // ret
    // Unlink arbitrary live mapping before munmap. Normal callers retain the
    // original syscall ABI; existing release wrappers preserve result RAX/RDX.
    code.label("recovery_temp_unmap")?;
    code.emit(&[0x57]); // push rdi
    code.emit(&[0x56]); // push rsi
    code.emit(&[0x52]); // push rdx
    code.emit(&[0x41, 0x50]); // push r8
    code.emit(&[0x41, 0x51]); // push r9
    code.emit(&[0x41, 0x52]); // push r10
    code.emit(&[0x48, 0x81, 0xef, 0x00, 0x10, 0x00, 0x00]); // sub rdi, 4096
    code.emit(&[0x4c, 0x8b, 0x07]); // mov r8, [rdi]
    code.emit(&[0x4c, 0x8b, 0x4f, 0x08]); // mov r9, [rdi+8]
    code.emit(&[0x4d, 0x85, 0xc0]); // test r8, r8
    code.emit_jcc_label(&[0x0f, 0x84], "recovery_temp_unmap_head");
    code.emit(&[0x4d, 0x89, 0x48, 0x08]); // mov [r8+8], r9
    code.emit_jmp_label("recovery_temp_unmap_next");
    code.label("recovery_temp_unmap_head")?;
    code.emit(&[0x4c, 0x89, 0x4d, 0x28]); // mov [rbp+40], r9
    code.label("recovery_temp_unmap_next")?;
    code.emit(&[0x4d, 0x85, 0xc9]); // test r9, r9
    code.emit_jcc_label(&[0x0f, 0x84], "recovery_temp_unmap_release");
    code.emit(&[0x4d, 0x89, 0x01]); // mov [r9], r8
    code.label("recovery_temp_unmap_release")?;
    code.emit(&[0x48, 0x8b, 0x77, 0x10]); // mov rsi, [rdi+16]
    code.emit(&[0xb8, 0x0b, 0x00, 0x00, 0x00]); // mov eax, 11
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x41, 0x5a]); // pop r10
    code.emit(&[0x41, 0x59]); // pop r9
    code.emit(&[0x41, 0x58]); // pop r8
    code.emit(&[0x5a]); // pop rdx
    code.emit(&[0x5e]); // pop rsi
    code.emit(&[0x5f]); // pop rdi
    code.emit(&[0xc3]); // ret
    code.label("recovery_temp_clean")?;
    code.emit(&[0x48, 0x8b, 0x7d, 0x28]); // mov rdi, [rbp+40]
    code.emit(&[0x48, 0x85, 0xff]); // test rdi, rdi
    code.emit_jcc_label(&[0x0f, 0x84], "recovery_temp_clean_done");
    code.emit(&[0x48, 0x81, 0xc7, 0x00, 0x10, 0x00, 0x00]); // add rdi, 4096
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x48, 0x85, 0xc0]); // test rax, rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_recovery_cleanup");
    code.emit_jmp_label("recovery_temp_clean");
    code.label("recovery_temp_clean_done")?;
    code.emit(&[0xc3]); // ret
    // Output record: previous0, root-frame8, slot-count16, selection-address24,
    // selection-value32, followed by the selected-bit snapshots in slot order.
    // No stack adjustment: normal exit and exceptional unwind own their RSP reset.
    code.label("recovery_output_pop")?;
    code.emit(&[0x48, 0x8b, 0x7d, 0x38]); // mov rdi, [rbp+56]
    code.emit(&[0x4c, 0x8b, 0x07]); // mov r8, [rdi]
    code.emit(&[0x4c, 0x89, 0x45, 0x38]); // mov [rbp+56], r8
    code.emit(&[0x4c, 0x8b, 0x47, 0x18]); // mov r8, [rdi+24]
    code.emit(&[0x48, 0x8b, 0x47, 0x20]); // mov rax, [rdi+32]
    code.emit(&[0x49, 0x89, 0x00]); // mov [r8], rax
    code.emit(&[0x4c, 0x8b, 0x47, 0x08]); // mov r8, [rdi+8]
    code.emit(&[0x49, 0x83, 0xc0, 0x10]); // add r8, 16
    code.emit(&[0x48, 0x8b, 0x4f, 0x10]); // mov rcx, [rdi+16]
    code.emit(&[0x48, 0x83, 0xc7, 0x28]); // add rdi, 40
    code.label("recovery_output_pop_loop")?;
    code.emit(&[0x48, 0x85, 0xc9]); // test rcx, rcx
    code.emit_jcc_label(&[0x0f, 0x84], "recovery_output_pop_done");
    code.emit(&[0x48, 0x8b, 0x07]); // mov rax, [rdi]
    code.emit(&[0x49, 0x89, 0x00]); // mov [r8], rax
    code.emit(&[0x48, 0x83, 0xc7, 0x08]); // add rdi, 8
    code.emit(&[0x49, 0x83, 0xc0, 0x40]); // add r8, 64
    code.emit(&[0x48, 0xff, 0xc9]); // dec rcx
    code.emit_jmp_label("recovery_output_pop_loop");
    code.label("recovery_output_pop_done")?;
    code.emit(&[0xc3]); // ret
    // The selected record owns every register below. The helper throw-time R14/R15
    // may be temporary scratch: recover the live fuel and depth from the record.
    code.label("recovery_resume")?;
    code.emit(&[0x48, 0x8b, 0x43, 0x48]); // mov rax, [rbx+72]
    code.emit(&[0x48, 0x89, 0x45, 0x30]); // mov [rbp+48], rax
    code.emit(&[0x4c, 0x8b, 0x30]); // mov r14, [rax]
    code.emit(&[0x4c, 0x8b, 0x7b, 0x40]); // mov r15, [rbx+64]
    code.emit(&[0x4c, 0x8b, 0x63, 0x30]); // mov r12, [rbx+48]
    code.emit(&[0x4c, 0x8b, 0x6b, 0x38]); // mov r13, [rbx+56]
    code.emit(&[0x4c, 0x8b, 0x5b, 0x10]); // mov r11, [rbx+16]
    code.emit(&[0x48, 0x8b, 0x63, 0x20]); // mov rsp, [rbx+32]
    code.emit(&[0x48, 0x8b, 0x5b, 0x28]); // mov rbx, [rbx+40]
    code.emit(&[0x41, 0xff, 0xe3]); // jmp r11
    code.label("recovery_raise")?;
    code.emit(&[0x41, 0x89, 0xfc]); // mov r12d, edi
    code.emit(&[0x89, 0xf9]); // mov ecx, edi
    code.emit(&[0x41, 0xb8, 0x01, 0x00, 0x00, 0x00]); // mov r8d, 1
    code.emit(&[0x49, 0xd3, 0xe0]); // shl r8, cl
    code.emit(&[0x48, 0x8b, 0x5d, 0x20]); // mov rbx, [rbp+32]
    code.label("recovery_search")?;
    code.emit(&[0x48, 0x85, 0xdb]); // test rbx, rbx
    code.emit_jcc_label(&[0x0f, 0x84], "recovery_unmatched");
    code.emit(&[0x4c, 0x85, 0x43, 0x08]); // test [rbx+8], r8
    code.emit_jcc_label(&[0x0f, 0x85], "recovery_selected");
    code.emit(&[0x48, 0x8b, 0x1b]); // mov rbx, [rbx]
    code.emit_jmp_label("recovery_search");
    code.label("recovery_selected")?;
    code.emit(&[0x48, 0x8b, 0x03]); // mov rax, [rbx]
    code.emit(&[0x48, 0x89, 0x45, 0x20]); // mov [rbp+32], rax
    code.emit_call_label("recovery_temp_clean");
    code.label("recovery_unwind_outputs")?;
    code.emit(&[0x48, 0x8b, 0x45, 0x38]); // mov rax, [rbp+56]
    code.emit(&[0x48, 0x3b, 0x43, 0x50]); // cmp rax, [rbx+80]
    code.emit_jcc_label(&[0x0f, 0x84], "recovery_publish");
    code.emit_call_label("recovery_output_pop");
    code.emit_jmp_label("recovery_unwind_outputs");
    code.label("recovery_publish")?;
    Ok(())
}

pub(super) fn emit_helpers(emitter: &mut Emitter) -> Result<(), String> {
    emit_storage_helpers(&mut emitter.code)?;
    for (index, category) in recovery::CATEGORIES.iter().enumerate() {
        let next = format!("recovery_category_next_{index}");
        emitter.code.emit(&[0x41, 0x83, 0xfc, index as u8]); // cmp r12d, index
        emitter.code.emit_jcc_label(&[0x0f, 0x85], &next);
        let code = recovery::code(category).unwrap();
        let data = emitter.data(code.as_bytes());
        emitter.code.emit_lea_rsi_label(&data);
        emitter
            .code
            .emit(&[0x48, 0x8b, 0x7b, 0x18, 0x48, 0x89, 0x37]); // destination=[record+24]; pointer=rsi
        emitter.code.emit(&[0x48, 0xc7, 0x47, 0x08]);
        emitter.code.emit(&(code.len() as u32).to_le_bytes());
        emitter.code.emit(&[0x48, 0xc7, 0x47, 0x18, 0, 0, 0, 0]); // numeric=0
        emitter.code.emit_jmp_label("recovery_resume");
        emitter.code.label(next)?;
    }
    emitter.code.label("recovery_unmatched")?;
    for (index, label) in [
        "int",
        "overflow",
        "divzero",
        "float",
        "bool",
        "text",
        "list",
        "index",
        "json",
        "json_path",
        "process",
        "timeout",
        "path",
        "bytes",
        "value",
    ]
    .iter()
    .enumerate()
    {
        emitter.code.emit(&[0x41, 0x83, 0xfc, index as u8]);
        emitter
            .code
            .emit_jcc_label(&[0x0f, 0x84], &format!("fatal_error_{label}"));
    }
    emitter.code.emit_jmp_label("error_recovery_cleanup");
    Ok(())
}

impl Emitter {
    pub(super) fn begin_recovery(&mut self, handler: &recovery::Handler, target: &str) {
        // The record lives below all owner locals and above any failed helper's
        // stack. Save owner state at a statement boundary, before scratch reuse.
        self.code
            .emit(&[0x48, 0x89, 0xe0, 0x48, 0x83, 0xec, HANDLER_BYTES as u8]); // old rsp; allocate
        self.code.emit(&[0x48, 0x89, 0x44, 0x24, 0x20]);
        self.code
            .emit(&[0x48, 0x8b, 0x45, 0x20, 0x48, 0x89, 0x04, 0x24]);
        self.code.emit(&[0x48, 0xc7, 0x44, 0x24, 0x08]);
        self.code.emit(&(handler.mask as u32).to_le_bytes());
        self.code.emit_lea_rsi_label(target);
        self.code.emit(&[0x48, 0x89, 0x74, 0x24, 0x10]);
        self.lea_rsi(self.slot(&handler.key));
        self.code.emit(&[0x48, 0x89, 0x74, 0x24, 0x18]);
        self.code.emit(&[
            0x48, 0x89, 0x5c, 0x24, 0x28, 0x4c, 0x89, 0x64, 0x24, 0x30, 0x4c, 0x89, 0x6c, 0x24,
            0x38, 0x4c, 0x89, 0x7c, 0x24, 0x40,
        ]);
        self.code
            .emit(&[0x48, 0x8b, 0x45, 0x30, 0x48, 0x89, 0x44, 0x24, 0x48]);
        self.code
            .emit(&[0x48, 0x8b, 0x45, 0x38, 0x48, 0x89, 0x44, 0x24, 0x50]);
        self.code.emit(&[0x48, 0x89, 0x65, 0x20]); // handler head=rsp
    }

    pub(super) fn pop_recovery(&mut self, count: usize) {
        for _ in 0..count {
            self.code.emit(&[
                0x48, 0x8b, 0x45, 0x20, 0x48, 0x8b, 0x08, 0x48, 0x89, 0x4d, 0x20, 0x48, 0x8b, 0x60,
                0x20,
            ]);
        }
    }

    pub(super) fn recovery_target(
        &mut self,
        layout: &recovery::Layout,
        from: usize,
        to: usize,
        target: String,
    ) -> Result<String, String> {
        let count = layout.exited(from, to)?;
        if count == 0 {
            return Ok(target);
        }
        let label = format!("recovery_jump_{}", self.recovery_jumps.len());
        self.recovery_jumps.push((label.clone(), target, count));
        Ok(label)
    }

    pub(super) fn emit_recovery_jumps(&mut self) -> Result<(), String> {
        for (label, target, count) in std::mem::take(&mut self.recovery_jumps) {
            self.code.label(label)?;
            self.pop_recovery(count);
            self.code.emit_jmp_label(&target);
        }
        Ok(())
    }

    pub(super) fn begin_function_context(&mut self, frame: usize) {
        let context = frame - FUNCTION_CONTEXT;
        self.code.emit(&[0x4c, 0x89, 0xb3]); // remaining fuel
        self.displacement(context);
        self.code.emit(&[0x48, 0x8b, 0x45, 0x30]);
        self.store_rax(context + 8);
        self.code.emit(&[0x48, 0x8b, 0x45, 0x20]);
        self.store_rax(context + 16);
        self.lea_rsi(context);
        self.code.emit(&[0x48, 0x89, 0x75, 0x30]);
    }

    pub(super) fn end_function_context(&mut self, frame: usize) {
        // Preserve the successful scalar/pointer result in RAX/RDX.
        self.load_rcx(frame - 8);
        self.code.emit(&[0x48, 0x89, 0x4d, 0x20]);
        self.load_rcx(frame - 16);
        self.code.emit(&[0x48, 0x89, 0x4d, 0x30]);
    }

    pub(super) fn begin_fuel_scope(&mut self, fuel: usize) {
        self.code.emit(&[0x48, 0x8b, 0x45, 0x30, 0x50, 0x49, 0xbe]); // save previous cell
        self.code.emit(&(fuel as u64).to_le_bytes());
        self.code.emit(&[0x41, 0x56, 0x48, 0x89, 0x65, 0x30]); // remaining fuel; current cell=rsp
    }

    pub(super) fn end_fuel_scope(&mut self) {
        self.code.emit(&[
            0x48, 0x8b, 0x44, 0x24, 0x08, 0x48, 0x89, 0x45, 0x30, 0x4c, 0x8b, 0x30, 0x48, 0x83,
            0xc4, 0x10,
        ]);
    }

    pub(super) fn begin_output_scope(&mut self) {
        let size = 40 + self.slots.len() * 8;
        self.code.emit(&[0x48, 0x81, 0xec]);
        self.code.emit(&(size as u32).to_le_bytes());
        self.code.emit(&[
            0x48, 0x8b, 0x45, 0x38, 0x48, 0x89, 0x04, 0x24, 0x48, 0x89, 0x5c, 0x24, 0x08,
        ]);
        self.code.emit(&[0x48, 0xc7, 0x44, 0x24, 0x10]);
        self.code.emit(&(self.slots.len() as u32).to_le_bytes());
        self.lea_rsi(self.selection_flag);
        self.code.emit(&[0x48, 0x89, 0x74, 0x24, 0x18]);
        self.load_rax(self.selection_flag);
        self.code.emit(&[0x48, 0x89, 0x44, 0x24, 0x20]);
        self.set(self.selection_flag, 0);
        for (index, slot) in self
            .slots
            .values()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .enumerate()
        {
            self.load_rax(slot + SELECTED);
            self.code.emit(&[0x48, 0x89, 0x84, 0x24]);
            self.code.emit(&((40 + index * 8) as u32).to_le_bytes());
            self.set(slot + SELECTED, 0);
        }
        self.code.emit(&[0x48, 0x89, 0x65, 0x38]);
    }

    pub(super) fn end_output_scope(&mut self) -> Result<(), String> {
        self.validate_selected()?;
        self.code.emit_call_label("recovery_output_pop");
        self.code.emit(&[0x48, 0x81, 0xc4]);
        self.code
            .emit(&((40 + self.slots.len() * 8) as u32).to_le_bytes());
        Ok(())
    }
}
