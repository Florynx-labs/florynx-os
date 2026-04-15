// =============================================================================
// Florynx Kernel — Task Structure
// =============================================================================
// Defines the kernel-level Task unit with its state machine and metadata.
// Tasks are the fundamental unit of execution in the Florynx scheduler.
// =============================================================================

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::{VirtAddr, PhysAddr};
use crate::security::capability::CapabilitySet;
use super::process::ProcessId;

/// Kernel stack size per task: 16 KiB.
pub const KERNEL_STACK_SIZE: usize = 16 * 1024;

/// FXSAVE/FXRSTOR state area: 512 bytes, must be 16-byte aligned.
#[repr(C, align(16))]
pub struct FpuState {
    pub data: [u8; 512],
}

impl FpuState {
    pub const fn new() -> Self {
        FpuState { data: [0u8; 512] }
    }

    /// Save current FPU/SSE state into this buffer.
    #[inline]
    pub unsafe fn save(&mut self) {
        core::arch::asm!(
            "fxsave [{0}]",
            in(reg) self.data.as_mut_ptr(),
            options(nostack, preserves_flags)
        );
    }

    /// Restore FPU/SSE state from this buffer.
    #[inline]
    pub unsafe fn restore(&self) {
        core::arch::asm!(
            "fxrstor [{0}]",
            in(reg) self.data.as_ptr(),
            options(nostack, preserves_flags)
        );
    }
}

/// Unique task identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

/// Global task ID counter.
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);
static USERMODE_JUMP_LOG_COUNT: AtomicU64 = AtomicU64::new(0);

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
    /// Task is sleeping/waiting and not runnable.
    Sleeping,
    /// Task exited and awaits reap/cleanup.
    Zombie,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskMode {
    Kernel,
    User(UserContext),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserContext {
    pub cr3: PhysAddr,
    pub rip: VirtAddr,
    pub rsp: VirtAddr,
    pub first_run: bool,
    /// Registers `rax` with this value on the first ring-3 jump.
    /// Used by fork() to make the child return 0.
    pub initial_rax: u64,
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
    /// Owned kernel stack (16 KiB). None = uses boot stack (idle/init tasks).
    pub kernel_stack: Option<Vec<u8>>,
    /// FPU/SSE state for lazy context switching.
    pub fpu_state: FpuState,
    /// Whether this task has ever used the FPU (lazy init).
    pub fpu_used: bool,
    /// Stack pointer (top of kernel stack)
    pub stack: Option<VirtAddr>,
    /// Time slice remaining (in timer ticks)
    pub time_slice: u64,
    /// Capability set for this task
    pub capabilities: CapabilitySet,
    /// Parent task identifier (if any)
    pub parent: Option<TaskId>,
    /// Exit status if task reached Zombie
    pub exit_code: Option<u64>,
    /// Absolute tick when sleeping task should wake
    pub wake_tick: Option<u64>,
    /// Owning process identifier
    pub process_id: ProcessId,
    /// Kernel or user execution mode
    pub mode: TaskMode,
    /// Pending signals bitfield (bit N-1 = signal N, per sig_bit()).
    pub pending_signals: u32,
}

impl Task {
    /// Create a new task with the given name and entry function.
    /// Allocates a 16 KiB kernel stack and sets up the initial switch frame.
    /// Also installs a guard page one page below the stack to catch overflow.
    pub fn new(name: &str, entry: fn()) -> Self {
        let mut stack_vec = Vec::<u8>::with_capacity(KERNEL_STACK_SIZE);
        stack_vec.resize(KERNEL_STACK_SIZE, 0u8);
        let stack_base_va = VirtAddr::new(stack_vec.as_ptr() as u64);
        let stack_top = unsafe {
            let top_ptr = stack_vec.as_mut_ptr().add(KERNEL_STACK_SIZE) as *mut u64;
            crate::arch::x86_64::asm_utils::init_task_stack(top_ptr, entry as u64)
        };
        let stack_top_va = VirtAddr::new(stack_top);
        // Guard page: one 4 KiB page immediately below the stack allocation.
        // Any stack overflow will fault here rather than silently corrupting heap.
        let guard_va = stack_base_va - 0x1000u64;
        crate::memory::paging::install_guard_page(guard_va);
        Task {
            id: TaskId::new(),
            name: String::from(name),
            state: TaskState::Ready,
            priority: TaskPriority::Normal,
            entry,
            run_count: 0,
            context: CpuContext { rsp: stack_top, ..CpuContext::new() },
            kernel_stack: Some(stack_vec),
            fpu_state: FpuState::new(),
            fpu_used: false,
            stack: Some(stack_top_va),
            time_slice: 10,
            capabilities: CapabilitySet::kernel(),
            parent: None,
            exit_code: None,
            wake_tick: None,
            process_id: ProcessId(0),
            mode: TaskMode::Kernel,
            pending_signals: 0,
        }
    }

    /// Create the idle/boot task.
    /// Does NOT allocate a kernel stack — the boot context's own stack is used.
    /// context.rsp starts at 0 and is populated by the first switch_to() call.
    pub fn idle(name: &str, entry: fn()) -> Self {
        Task {
            id: TaskId::new(),
            name: String::from(name),
            state: TaskState::Ready,
            priority: TaskPriority::Low,
            entry,
            run_count: 0,
            context: CpuContext::new(), // rsp=0, filled in by first switch_to
            kernel_stack: None,
            fpu_state: FpuState::new(),
            fpu_used: false,
            stack: None,
            time_slice: 5,
            capabilities: CapabilitySet::kernel(),
            parent: None,
            exit_code: None,
            wake_tick: None,
            process_id: ProcessId(0),
            mode: TaskMode::Kernel,
            pending_signals: 0,
        }
    }

    /// Create a new task with a specific priority.
    pub fn with_priority(name: &str, entry: fn(), priority: TaskPriority) -> Self {
        let mut t = Task::new(name, entry);
        t.priority = priority;
        t.time_slice = match priority {
            TaskPriority::Low => 5,
            TaskPriority::Normal => 10,
            TaskPriority::High => 20,
            TaskPriority::Realtime => 50,
        };
        t
    }

    /// Create a user task with explicit userspace execution context.
    pub fn new_user(
        name: &str,
        process_id: ProcessId,
        user_ctx: UserContext,
        capabilities: CapabilitySet,
    ) -> Self {
        let mut task = Task::new(name, user_placeholder_entry);
        task.process_id = process_id;
        task.capabilities = capabilities;
        task.mode = TaskMode::User(user_ctx);
        task
    }

    /// Check if this task has a specific capability.
    pub fn has_cap(&self, cap: crate::security::capability::Capability) -> bool {
        self.capabilities.has(cap)
    }
}

fn user_placeholder_entry() {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Transition to User Mode (Ring 3) and execute the given entry point.
/// `initial_rax` is the value visible in RAX when ring-3 code starts
/// (pass 0 for fresh processes; fork child uses 0, parent uses child PID).
/// This function never returns.
pub unsafe fn jump_to_user_mode(
    entry: VirtAddr,
    user_stack: VirtAddr,
    page_table: Option<PhysAddr>,
    initial_rax: u64,
) -> ! {
    let user_data = crate::arch::x86_64::gdt::GDT.1.user_data_selector.0 | 3; // RPL 3
    let user_code = crate::arch::x86_64::gdt::GDT.1.user_code_selector.0 | 3;
    
    // Disable interrupts during the CR3 switch → iretq window to avoid
    // a timer IRQ hitting while the new page table is active but we are
    // still in Ring 0 with a partially-prepared stack frame.
    // iretq will restore RFLAGS with IF=1 so interrupts resume in Ring 3.
    core::arch::asm!("cli");
    if let Some(pt) = page_table {
        core::arch::asm!("mov cr3, {}", in(reg) pt.as_u64());
    }
    if USERMODE_JUMP_LOG_COUNT.fetch_add(1, Ordering::AcqRel) == 0 {
        crate::serial_println!(
            "[task] jump_to_user_mode entry=0x{:x} stack=0x{:x} cr3=0x{:x} rax=0x{:x}",
            entry.as_u64(),
            user_stack.as_u64(),
            page_table.map(|p| p.as_u64()).unwrap_or(0),
            initial_rax
        );
    }

    // iretq frame (CPU pops in this order: RIP, CS, RFLAGS, RSP, SS).
    // We push in reverse: SS first, then RSP, RFLAGS, CS, RIP.
    // After the frame is built we set rax = initial_rax (fork child returns 0).
    core::arch::asm!(
        "mov ds, {0:x}",  // {0} = user_data (SS and DS selector)
        "mov es, {0:x}",
        "mov fs, {0:x}",
        "mov gs, {0:x}",
        "push {0}",        // SS
        "push {1}",        // user RSP
        "push 0x202",      // RFLAGS: IF=1, reserved bit 1=1
        "push {2}",        // user CS
        "push {3}",        // user RIP
        "mov rax, {4}",    // initial_rax for the new process
        "iretq",
        in(reg) user_data as u64,    // {0}
        in(reg) user_stack.as_u64(), // {1}
        in(reg) user_code as u64,    // {2}
        in(reg) entry.as_u64(),      // {3}
        in(reg) initial_rax,         // {4}
        options(noreturn)
    )
}
