// =============================================================================
// Florynx Kernel — CPU Context for Context Switching
// =============================================================================
// Stores and restores CPU register state for cooperative/preemptive
// context switching between tasks.
// =============================================================================

/// Saved CPU context for a task.
/// In a full implementation, this would save all general-purpose registers,
/// segment selectors, and FPU/SSE state.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpuContext {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cs: u64,
    pub ss: u64,
}

impl CpuContext {
    /// Create a zeroed-out context.
    pub const fn empty() -> Self {
        CpuContext {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0, rflags: 0, cs: 0, ss: 0,
        }
    }

    /// Create a context for a new kernel task with the given entry point and stack.
    pub fn new_kernel_task(entry_point: u64, stack_top: u64) -> Self {
        CpuContext {
            rip: entry_point,
            rsp: stack_top,
            rflags: 0x202, // IF flag set (interrupts enabled)
            cs: 0x08,      // Kernel code segment
            ss: 0x10,      // Kernel data segment
            ..Self::empty()
        }
    }
}
