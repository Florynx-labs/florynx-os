// =============================================================================
// Florynx Kernel — Task Structure
// =============================================================================
// Defines the kernel-level Task unit with its state machine and metadata.
// Tasks are the fundamental unit of execution in the Florynx scheduler.
// =============================================================================

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::{VirtAddr, PhysAddr};
use crate::security::capability::CapabilitySet;

/// Unique task identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

/// Global task ID counter.
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

impl TaskId {
    /// Generate the next unique task ID.
    pub fn new() -> Self {
        TaskId(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Task execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to run.
    Ready,
    /// Task is currently running on a CPU.
    Running,
    /// Task is blocked waiting for an event.
    Blocked,
    /// Task has terminated.
    Terminated,
}

/// Priority level for task scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Realtime,
}

/// CPU context saved during context switch
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CpuContext {
    pub rsp: u64,  // Stack pointer
    pub rbp: u64,  // Base pointer
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rflags: u64,
}

impl CpuContext {
    pub const fn new() -> Self {
        CpuContext {
            rsp: 0,
            rbp: 0,
            rbx: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rflags: 0x202, // IF (interrupt flag) set
        }
    }
}

/// A kernel task — the basic unit of work in the scheduler.
pub struct Task {
    /// Unique identifier.
    pub id: TaskId,
    /// Human-readable name.
    pub name: String,
    /// Current state.
    pub state: TaskState,
    /// Scheduling priority.
    pub priority: TaskPriority,
    /// The function this task executes.
    pub entry: fn(),
    /// Number of times this task has been scheduled.
    pub run_count: u64,
    /// CPU context (saved registers)
    pub context: CpuContext,
    /// Stack pointer (top of kernel stack)
    pub stack: Option<VirtAddr>,
    /// Time slice remaining (in timer ticks)
    pub time_slice: u64,
    /// Capability set for this task
    pub capabilities: CapabilitySet,
}

impl Task {
    /// Create a new task with the given name and entry function.
    pub fn new(name: &str, entry: fn()) -> Self {
        Task {
            id: TaskId::new(),
            name: String::from(name),
            state: TaskState::Ready,
            priority: TaskPriority::Normal,
            entry,
            run_count: 0,
            context: CpuContext::new(),
            stack: None,
            time_slice: 10, // Default 10 ticks
            capabilities: CapabilitySet::kernel(), // Kernel tasks get all caps
        }
    }

    /// Create a new task with a specific priority.
    pub fn with_priority(name: &str, entry: fn(), priority: TaskPriority) -> Self {
        Task {
            id: TaskId::new(),
            name: String::from(name),
            state: TaskState::Ready,
            priority,
            entry,
            run_count: 0,
            context: CpuContext::new(),
            stack: None,
            time_slice: match priority {
                TaskPriority::Low => 5,
                TaskPriority::Normal => 10,
                TaskPriority::High => 20,
                TaskPriority::Realtime => 50,
            },
            capabilities: CapabilitySet::kernel(),
        }
    }

    /// Check if this task has a specific capability.
    pub fn has_cap(&self, cap: crate::security::capability::Capability) -> bool {
        self.capabilities.has(cap)
    }
}

/// Transition to User Mode (Ring 3) and execute the given entry point.
/// This function never returns.
pub unsafe fn jump_to_user_mode(entry: VirtAddr, user_stack: VirtAddr, page_table: Option<PhysAddr>) -> ! {
    let user_data = crate::arch::x86_64::gdt::GDT.1.user_data_selector.0 | 3; // RPL 3
    let user_code = crate::arch::x86_64::gdt::GDT.1.user_code_selector.0 | 3;
    
    if let Some(pt) = page_table {
        core::arch::asm!("mov cr3, {}", in(reg) pt.as_u64());
    }

    // iretq expects the stack to look like:
    // [SS, RSP, RFLAGS, CS, RIP]
    core::arch::asm!(
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "push rax",       // SS
        "push rsi",       // RSP
        "push 0x202",     // RFLAGS (IF=1)
        "push rdx",       // CS
        "push rdi",       // RIP
        "iretq",
        in("ax") user_data,
        in("rdx") user_code,
        in("rdi") entry.as_u64(),
        in("rsi") user_stack.as_u64(),
        options(noreturn)
    )
}
