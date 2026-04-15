// =============================================================================
// Florynx Kernel — Preemptive Round-Robin Scheduler
// =============================================================================
// Production-level preemptive scheduler with:
// - Time-slice based scheduling
// - Priority levels (Low, Normal, High, Realtime)
// - Process states (Ready, Running, Blocked, Terminated)
// - Idle task
// =============================================================================

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use super::task::{Task, TaskState, TaskId, TaskPriority, TaskMode, UserContext};
use crate::security::capability::CapabilitySet;
use super::process::{Process, ProcessId};

/// Task ID of the task that currently owns the FPU/SSE hardware state.
/// u64::MAX means no owner (FPU is in clean/reset state).
static FPU_OWNER: AtomicU64 = AtomicU64::new(u64::MAX);

lazy_static! {
    static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}
static FIRST_USER_JUMP_LOGGED: AtomicBool = AtomicBool::new(false);

/// Preemptive scheduler with priority-based time slices
struct Scheduler {
    /// All tasks indexed by slot
    tasks: Vec<Option<Task>>,
    /// Ready queue (task slot indices)
    ready_queue: VecDeque<usize>,
    /// Currently running task index
    current_task: Option<usize>,
    /// Idle task index
    idle_task: Option<usize>,
    /// Total scheduling rounds
    rounds: u64,
    /// Scheduler enabled
    enabled: bool,
    /// Current time slice remaining
    current_time_slice: u64,
}

impl Scheduler {
    const fn new() -> Self {
        Scheduler {
            tasks: Vec::new(),
            ready_queue: VecDeque::new(),
            current_task: None,
            idle_task: None,
            rounds: 0,
            enabled: false,
            current_time_slice: 0,
        }
    }

    fn add_task_internal(&mut self, mut task: Task) -> usize {
        task.parent = self.current().map(|t| t.id);
        for (i, slot) in self.tasks.iter_mut().enumerate() {
            if slot.is_none() {
                task.state = TaskState::Ready;
                *slot = Some(task);
                self.ready_queue.push_back(i);
                return i;
            }
        }
        let idx = self.tasks.len();
        task.state = TaskState::Ready;
        self.tasks.push(Some(task));
        self.ready_queue.push_back(idx);
        idx
    }

    fn pick_next_task(&mut self) -> Option<usize> {
        while let Some(idx) = self.ready_queue.pop_front() {
            if let Some(ref task) = self.tasks[idx] {
                if task.state == TaskState::Ready {
                    return Some(idx);
                }
            }
        }
        self.idle_task
    }

    fn schedule_next(&mut self) -> Option<(usize, usize)> {
        let next_idx = self.pick_next_task()?;
        let prev_idx = self.current_task;

        if let Some(prev) = prev_idx {
            if let Some(ref mut task) = self.tasks[prev] {
                if task.state == TaskState::Running {
                    task.state = TaskState::Ready;
                    self.ready_queue.push_back(prev);
                }
            }
        }

        if let Some(ref mut task) = self.tasks[next_idx] {
            task.state = TaskState::Running;
            task.run_count += 1;
            self.current_time_slice = task.time_slice;
        }

        self.current_task = Some(next_idx);
        self.rounds += 1;

        prev_idx.map(|prev| (prev, next_idx))
    }

    fn block_current(&mut self) {
        if let Some(idx) = self.current_task {
            if let Some(ref mut task) = self.tasks[idx] {
                task.state = TaskState::Sleeping;
            }
        }
    }

    fn wake_task(&mut self, idx: usize) {
        if let Some(ref mut task) = self.tasks[idx] {
            if task.state == TaskState::Sleeping {
                task.state = TaskState::Ready;
                task.wake_tick = None;
                self.ready_queue.push_back(idx);
            }
        }
    }

    fn terminate_current(&mut self, exit_code: u64) {
        if let Some(idx) = self.current_task {
            if let Some(ref mut task) = self.tasks[idx] {
                task.state = TaskState::Zombie;
                task.exit_code = Some(exit_code);
                crate::serial_println!(
                    "[scheduler] task '{}' became zombie (code={})",
                    task.name,
                    exit_code
                );
            }
            self.current_task = None;
        }
    }

    fn current(&self) -> Option<&Task> {
        self.current_task.and_then(|idx| self.tasks[idx].as_ref())
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Initialize the scheduler with an idle task
pub fn init() {
    let mut sched = SCHEDULER.lock();
    // Idle task does NOT get a pre-initialized stack: the boot context IS idle.
    // context.rsp=0 will be overwritten by the first switch_to() call.
    let idle = Task::idle("idle", idle_task_fn);
    let idle_idx = sched.tasks.len();
    sched.tasks.push(Some(idle));
    if let Some(ref mut t) = sched.tasks[idle_idx] {
        t.state = TaskState::Running; // boot context is running
    }
    sched.idle_task = Some(idle_idx);
    sched.current_task = Some(idle_idx); // boot context = idle task
    sched.enabled = true;
    crate::serial_println!("[scheduler] initialized (idle=boot context, idx={})", idle_idx);
}

fn idle_task_fn() {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Spawn a new task with normal priority
pub fn spawn(name: &str, entry: fn()) -> TaskId {
    let task = Task::new(name, entry);
    let id = task.id;
    let mut sched = SCHEDULER.lock();
    let idx = sched.add_task_internal(task);
    crate::serial_println!("[scheduler] spawned task '{}' (id={}, idx={})", name, id.0, idx);
    id
}

/// Spawn a task with specific priority
pub fn spawn_with_priority(name: &str, entry: fn(), priority: TaskPriority) -> TaskId {
    let task = Task::with_priority(name, entry, priority);
    let id = task.id;
    let mut sched = SCHEDULER.lock();
    let _idx = sched.add_task_internal(task);
    crate::serial_println!("[scheduler] spawned task '{}' (id={}, priority={:?})", name, id.0, priority);
    id
}

/// Spawn a user process task with explicit userspace context.
pub fn spawn_user_process(
    name: &str,
    user_cr3: x86_64::PhysAddr,
    owns_page_table: bool,
    user_entry: x86_64::VirtAddr,
    user_stack_top: x86_64::VirtAddr,
    capabilities: CapabilitySet,
) -> TaskId {
    let parent = ProcessId(current_task_id().map(|t| t.0).unwrap_or(0));
    let mut process = Process::new(name, parent);
    process.page_table = Some(user_cr3);
    process.owns_page_table = owns_page_table;
    process.user_regions.push((user_entry.as_u64() & !0xFFF, 0x1000));
    process.user_regions.push((user_stack_top.as_u64().saturating_sub(0x1000), 0x1000));
    let pid = crate::process::process::register_process(process);

    let user_ctx = UserContext {
        cr3: user_cr3,
        rip: user_entry,
        rsp: user_stack_top,
        first_run: true,
        initial_rax: 0,
    };
    let task = Task::new_user(name, pid, user_ctx, capabilities);
    let id = task.id;
    let mut sched = SCHEDULER.lock();
    let idx = sched.add_task_internal(task);
    drop(sched);
    crate::process::process::attach_task_to_process(pid, id);
    crate::serial_println!(
        "[scheduler] spawned user process '{}' (pid={}, tid={}, idx={})",
        name,
        pid.0,
        id.0,
        idx
    );
    id
}

/// Enable the scheduler
pub fn enable() {
    let mut sched = SCHEDULER.lock();
    sched.enabled = true;
    crate::serial_println!("[scheduler] enabled");
}

/// Disable the scheduler
pub fn disable() {
    let mut sched = SCHEDULER.lock();
    sched.enabled = false;
    crate::serial_println!("[scheduler] disabled");
}

/// Called by timer interrupt — handle time slice and scheduling
pub fn timer_tick() {
    if let Some(mut sched) = SCHEDULER.try_lock() {
        if !sched.enabled {
            return;
        }

        // Wake sleeping tasks whose timeout elapsed.
        let now = crate::drivers::timer::pit::get_ticks();
        for idx in 0..sched.tasks.len() {
            if let Some(ref mut task) = sched.tasks[idx] {
                if task.state == TaskState::Sleeping {
                    if let Some(wake_tick) = task.wake_tick {
                        if wake_tick <= now {
                            task.state = TaskState::Ready;
                            task.wake_tick = None;
                            sched.ready_queue.push_back(idx);
                        }
                    }
                }
            }
        }

        if sched.current_time_slice > 0 {
            sched.current_time_slice -= 1;
        }
        // Note: time-slice expiry is advisory. The actual task switch happens
        // at the next cooperative do_context_switch() call (yield/sleep/syscall).
        // Preemptive switching from the timer IRQ requires IRQ-stack surgery
        // and is deferred to Phase 1.2.
    }
}

/// Block the current task
pub fn block() {
    let mut sched = SCHEDULER.lock();
    sched.block_current();
}

/// Wake a task by ID
pub fn wake(id: TaskId) {
    let mut sched = SCHEDULER.lock();
    for (idx, task_opt) in sched.tasks.iter().enumerate() {
        if let Some(ref task) = task_opt {
            if task.id == id {
                sched.wake_task(idx);
                break;
            }
        }
    }
}

/// Terminate the current task
pub fn exit() {
    exit_with_code(0);
}

/// Terminate current task with explicit exit code and leave as zombie.
pub fn exit_with_code(exit_code: u64) {
    {
        let mut sched = SCHEDULER.lock();
        sched.terminate_current(exit_code);
    }
    // Switch to next runnable task — never returns to caller.
    do_context_switch();
    // Unreachable, but in case no switch happened:
    loop { x86_64::instructions::hlt(); }
}

/// Handle a user-mode page fault by terminating the current task and
/// immediately selecting the next runnable task.
pub fn handle_user_page_fault(fault_addr: u64, error_bits: u64) {
    let mut sched = SCHEDULER.lock();
    if let Some(idx) = sched.current_task {
        if let Some(ref mut task) = sched.tasks[idx] {
            task.state = TaskState::Zombie;
            task.exit_code = Some(139);
            crate::serial_println!(
                "[scheduler] user page fault: task '{}' (id={}) addr=0x{:x} err=0x{:x}",
                task.name,
                task.id.0,
                fault_addr,
                error_bits
            );
        }
        sched.current_task = None;
    }
    let _ = sched.schedule_next();
}

/// Yield CPU to next task (cooperative context switch).
pub fn yield_now() {
    do_context_switch();
}

/// Return the CR3 (page-table base) of the currently running user task.
/// Returns `None` if the current task is a kernel task.
pub fn current_task_cr3() -> Option<x86_64::PhysAddr> {
    let sched = SCHEDULER.lock();
    let task = sched.current()?;
    match task.mode {
        TaskMode::User(ctx) => Some(ctx.cr3),
        TaskMode::Kernel => None,
    }
}

/// Spawn a child task for fork(). The child uses `initial_rax = 0` so that
/// when it is first scheduled and performs its ring-3 jump, fork() returns 0.
pub fn spawn_fork_child(
    child_cr3: x86_64::PhysAddr,
    owns_page_table: bool,
    user_rip: x86_64::VirtAddr,
    user_rsp: x86_64::VirtAddr,
    capabilities: CapabilitySet,
) -> TaskId {
    let parent_pid = ProcessId(current_task_id().map(|t| t.0).unwrap_or(0));
    let mut process = Process::new("fork-child", parent_pid);
    process.page_table = Some(child_cr3);
    process.owns_page_table = owns_page_table;
    let pid = crate::process::process::register_process(process);

    let user_ctx = UserContext {
        cr3: child_cr3,
        rip: user_rip,
        rsp: user_rsp,
        first_run: true,
        initial_rax: 0,  // child: fork() returns 0
    };
    let task = Task::new_user("fork-child", pid, user_ctx, capabilities);
    let id = task.id;
    let mut sched = SCHEDULER.lock();
    let idx = sched.add_task_internal(task);
    drop(sched);
    crate::process::process::attach_task_to_process(pid, id);
    crate::serial_println!(
        "[scheduler] fork child spawned (tid={}, pid={}, idx={})",
        id.0, pid.0, idx
    );
    id
}

/// Get current task ID
pub fn current_task_id() -> Option<TaskId> {
    let sched = SCHEDULER.lock();
    sched.current().map(|t| t.id)
}

/// Get capabilities of current task, if one is running.
pub fn current_task_capabilities() -> Option<CapabilitySet> {
    let sched = SCHEDULER.lock();
    sched.current().map(|t| t.capabilities)
}

/// Put current task to sleep for a number of timer ticks (cooperative).
pub fn sleep_current(ticks: u64) {
    {
        let mut sched = SCHEDULER.lock();
        let now = crate::drivers::timer::pit::get_ticks();
        if let Some(idx) = sched.current_task {
            if let Some(ref mut task) = sched.tasks[idx] {
                task.state = TaskState::Sleeping;
                task.wake_tick = Some(now.saturating_add(ticks));
            }
        }
    }
    // Switch to next runnable task.
    do_context_switch();
}

/// Wait for any zombie child of current task.
/// Returns (child_id, exit_code) if one is reaped.
pub fn wait_any_child() -> Option<(TaskId, u64)> {
    let mut sched = SCHEDULER.lock();
    let parent_id = sched.current().map(|t| t.id)?;
    for slot in sched.tasks.iter_mut() {
        if let Some(task) = slot {
            if task.parent == Some(parent_id) && task.state == TaskState::Zombie {
                let id = task.id;
                let code = task.exit_code.unwrap_or(0);
                let cleanup = crate::process::process::cleanup_task_resources(id);
                if cleanup.closed_fds > 0 {
                    crate::serial_println!(
                        "[scheduler] reaped task {} and closed {} fd(s)",
                        id.0,
                        cleanup.closed_fds
                    );
                }
                if cleanup.released_process_links > 0
                    || cleanup.released_user_regions > 0
                    || cleanup.released_page_tables > 0
                {
                    crate::serial_println!(
                        "[scheduler] task {} cleanup: links={} regions={} tables={}",
                        id.0,
                        cleanup.released_process_links,
                        cleanup.released_user_regions,
                        cleanup.released_page_tables
                    );
                }
                *slot = None; // reap
                return Some((id, code));
            }
        }
    }
    None
}

/// Returns true if current task has at least one child task.
pub fn has_any_child() -> bool {
    let sched = SCHEDULER.lock();
    let parent_id = match sched.current().map(|t| t.id) {
        Some(id) => id,
        None => return false,
    };
    sched.tasks.iter().any(|slot| {
        if let Some(task) = slot {
            task.parent == Some(parent_id)
        } else {
            false
        }
    })
}

/// Perform a cooperative context switch to the next ready task.
///
/// Safe to call with interrupts enabled (disables internally for the switch).
/// Saves the current task's RSP via switch_to, then restores next task's.
/// If there is no other runnable task, returns immediately.
pub fn do_context_switch() {
    let was_enabled = x86_64::instructions::interrupts::are_enabled();
    x86_64::instructions::interrupts::disable();

    // Phase 1: collect (prev_rsp_ptr, next_rsp) while holding the lock.
    // SAFETY: The Scheduler is a global static and its task Vec does not
    // reallocate between here and switch_to() because interrupts are disabled
    // and we are single-CPU. The pointer to context.rsp remains valid.
    let switch_pair: Option<(*mut u64, u64)> = {
        let mut sched = SCHEDULER.lock();

        let prev_idx = match sched.current_task {
            Some(idx) => idx,
            None => {
                if was_enabled { x86_64::instructions::interrupts::enable(); }
                return;
            }
        };

        // If prev task is still Running, put it back as Ready.
        if let Some(ref mut t) = sched.tasks[prev_idx] {
            if t.state == TaskState::Running {
                t.state = TaskState::Ready;
                sched.ready_queue.push_back(prev_idx);
            }
        }

        let next_idx = match sched.pick_next_task() {
            Some(idx) => idx,
            None => {
                // Restore prev state and abort switch.
                if let Some(ref mut t) = sched.tasks[prev_idx] {
                    t.state = TaskState::Running;
                }
                if was_enabled { x86_64::instructions::interrupts::enable(); }
                return;
            }
        };

        if next_idx == prev_idx {
            // Only one runnable task — stay on it.
            if let Some(ref mut t) = sched.tasks[prev_idx] {
                t.state = TaskState::Running;
            }
            if was_enabled { x86_64::instructions::interrupts::enable(); }
            return;
        }

        if let Some(ref mut t) = sched.tasks[next_idx] {
            t.state = TaskState::Running;
            t.run_count += 1;
            sched.current_time_slice = t.time_slice;
        }
        sched.current_task = Some(next_idx);
        sched.rounds += 1;

        let prev_rsp_ptr = &mut sched.tasks[prev_idx].as_mut().unwrap().context.rsp as *mut u64;
        let next_rsp = sched.tasks[next_idx].as_ref().unwrap().context.rsp;

        Some((prev_rsp_ptr, next_rsp))
        // Lock drops here — safe: switch_to only uses the raw RSP values.
    };

    if let Some((prev_rsp_ptr, next_rsp)) = switch_pair {
        // Signal the FPU lazy-switch mechanism that a task boundary occurred.
        crate::arch::x86_64::cpu::set_task_switched();

        // Keep interrupts DISABLED during switch_to — re-enable after the stack
        // swap so no timer IRQ fires between lock-drop and switch_to().
        // SAFETY: interrupts disabled on single-CPU; stacks are heap-allocated
        // for the lifetime of the Task; prev_rsp_ptr points into a static global.
        unsafe {
            crate::arch::x86_64::asm_utils::switch_to(next_rsp, prev_rsp_ptr);
        }
        // Execution resumes here when we are switched BACK to this context.
        if was_enabled { x86_64::instructions::interrupts::enable(); }
    } else {
        if was_enabled { x86_64::instructions::interrupts::enable(); }
    }
}

/// Called from the #NM (Device Not Available) exception handler.
/// Implements lazy FPU context switching: save old owner's state, restore ours.
pub fn handle_fpu_fault() {
    let mut sched = SCHEDULER.lock();
    let current_idx = match sched.current_task {
        Some(idx) => idx,
        None => return,
    };
    let current_id = match sched.tasks[current_idx].as_ref() {
        Some(t) => t.id.0,
        None => return,
    };

    let old_owner = FPU_OWNER.swap(current_id, Ordering::AcqRel);
    if old_owner == current_id {
        return; // Same task — just allow the instruction.
    }

    // Save the old FPU owner's state.
    if old_owner != u64::MAX {
        for slot in sched.tasks.iter_mut() {
            if let Some(task) = slot {
                if task.id.0 == old_owner {
                    unsafe { task.fpu_state.save(); }
                    break;
                }
            }
        }
    }

    // Restore current task's FPU state (or init if first use).
    if let Some(ref mut task) = sched.tasks[current_idx] {
        if task.fpu_used {
            unsafe { task.fpu_state.restore(); }
        } else {
            unsafe {
                core::arch::asm!("fninit", options(nomem, nostack, preserves_flags));
            }
            task.fpu_used = true;
        }
    }
}

/// Send signal `signum` to task `id`.
/// For fatal signals (SIGKILL, SIGTERM, etc.) delivered to the CURRENT task,
/// the task is immediately terminated and never returns from this function.
/// For other tasks, the signal is queued in `pending_signals` for later
/// delivery at the next `deliver_pending_signals()` call.
/// Returns false if no task with `id` was found.
pub fn send_signal(id: TaskId, signum: u32) -> bool {
    use crate::process::signal::{is_fatal_default, sig_bit, signal_exit_code};
    if signum == 0 || signum > crate::process::signal::SIGMAX {
        return false;
    }
    let bit = sig_bit(signum);
    let mut sched = SCHEDULER.lock();
    let current_id = sched.current().map(|t| t.id);
    for slot in sched.tasks.iter_mut() {
        if let Some(task) = slot {
            if task.id == id {
                if is_fatal_default(signum) {
                    if Some(id) == current_id {
                        // Self-signal: mark zombie and switch away.
                        task.state = TaskState::Zombie;
                        task.exit_code = Some(signal_exit_code(signum));
                        crate::serial_println!(
                            "[signal] task '{}' self-terminated by sig{}",
                            task.name, signum
                        );
                        drop(sched);
                        do_context_switch();
                        return true;
                    } else {
                        // Remote fatal signal: terminate immediately.
                        task.state = TaskState::Zombie;
                        task.exit_code = Some(signal_exit_code(signum));
                        task.wake_tick = None;
                        crate::serial_println!(
                            "[signal] task '{}' killed by sig{}",
                            task.name, signum
                        );
                        return true;
                    }
                } else {
                    // Non-fatal signal: queue for later delivery.
                    task.pending_signals |= bit;
                    return true;
                }
            }
        }
    }
    false
}

/// Check and deliver any pending signals for the current task.
/// Must be called from safe kernel context (not holding scheduler lock).
/// If a fatal signal is pending, the current task is terminated.
pub fn deliver_pending_signals() {
    use crate::process::signal::{is_fatal_default, signal_exit_code, SIGMAX};
    let mut fatal_triggered = false;
    {
        let mut sched = SCHEDULER.lock();
        let idx = match sched.current_task {
            Some(i) => i,
            None => return,
        };
        if let Some(ref mut task) = sched.tasks[idx] {
            let pending = task.pending_signals;
            if pending == 0 { return; }
            // Deliver the lowest-numbered pending signal.
            for sig in 1..=SIGMAX {
                if pending & (1 << (sig - 1)) != 0 {
                    task.pending_signals &= !(1 << (sig - 1));
                    if is_fatal_default(sig) {
                        task.state = TaskState::Zombie;
                        task.exit_code = Some(signal_exit_code(sig));
                        crate::serial_println!(
                            "[signal] task '{}' terminated by pending sig{}",
                            task.name, sig
                        );
                        fatal_triggered = true;
                    }
                    break; // One signal per call.
                }
            }
        }
    }
    if fatal_triggered {
        do_context_switch();
    }
}

/// Wait for a specific child task (by ID) to become zombie.
/// Returns (exit_code) on success, or None if not found / not a child.
pub fn wait_child_pid(child_id: TaskId) -> Option<u64> {
    let mut sched = SCHEDULER.lock();
    let parent_id = sched.current().map(|t| t.id)?;
    for slot in sched.tasks.iter_mut() {
        if let Some(task) = slot {
            if task.id == child_id
                && task.parent == Some(parent_id)
                && task.state == TaskState::Zombie
            {
                let code = task.exit_code.unwrap_or(0);
                let cleanup = crate::process::process::cleanup_task_resources(child_id);
                if cleanup.closed_fds > 0 || cleanup.released_process_links > 0 {
                    crate::serial_println!(
                        "[scheduler] waitpid reaped task {} code={}",
                        child_id.0, code
                    );
                }
                *slot = None;
                return Some(code);
            }
        }
    }
    None
}

/// Kill task by ID by transitioning it to zombie.
pub fn kill(id: TaskId, exit_code: u64) -> bool {
    let mut sched = SCHEDULER.lock();
    for (idx, slot) in sched.tasks.iter_mut().enumerate() {
        if let Some(task) = slot {
            if task.id == id {
                task.state = TaskState::Zombie;
                task.exit_code = Some(exit_code);
                task.wake_tick = None;
                if sched.current_task == Some(idx) {
                    sched.current_task = None;
                    let _ = sched.schedule_next();
                }
                return true;
            }
        }
    }
    false
}

/// Get scheduler statistics
pub fn stats() -> SchedulerStats {
    let sched = SCHEDULER.lock();
    SchedulerStats {
        total_tasks: sched.tasks.iter().filter(|t| t.is_some()).count(),
        ready_tasks: sched.ready_queue.len(),
        current_task: sched.current().map(|t| t.name.clone()),
        rounds: sched.rounds,
    }
}

/// If the current task is a first-run user task, perform Ring3 transition.
/// Returns true when a transition path was attempted.
pub fn run_current_user_first_run() -> bool {
    let mut sched = SCHEDULER.lock();
    let idx = match sched.current_task {
        Some(i) => i,
        None => return false,
    };
    let task = match sched.tasks[idx].as_mut() {
        Some(t) => t,
        None => return false,
    };
    let mut user_ctx = match task.mode {
        TaskMode::User(ctx) => ctx,
        TaskMode::Kernel => return false,
    };
    if !user_ctx.first_run {
        return false;
    }
    user_ctx.first_run = false;
    task.mode = TaskMode::User(user_ctx);
    if !FIRST_USER_JUMP_LOGGED.swap(true, Ordering::AcqRel) {
        crate::serial_println!(
            "[scheduler] first user jump tid={} rip=0x{:x} rsp=0x{:x} cr3=0x{:x}",
            task.id.0,
            user_ctx.rip.as_u64(),
            user_ctx.rsp.as_u64(),
            user_ctx.cr3.as_u64()
        );
    }
    drop(sched);
    unsafe {
        super::task::jump_to_user_mode(
            user_ctx.rip,
            user_ctx.rsp,
            Some(user_ctx.cr3),
            user_ctx.initial_rax,
        );
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub total_tasks: usize,
    pub ready_tasks: usize,
    pub current_task: Option<alloc::string::String>,
    pub rounds: u64,
}
