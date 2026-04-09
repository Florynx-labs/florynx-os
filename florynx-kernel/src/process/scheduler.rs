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
use lazy_static::lazy_static;
use spin::Mutex;

use super::task::{Task, TaskState, TaskId, TaskPriority};
use crate::security::capability::CapabilitySet;

lazy_static! {
    static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

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
    let idle = Task::new("idle", idle_task_fn);
    let idle_idx = sched.add_task_internal(idle);
    sched.idle_task = Some(idle_idx);
    crate::serial_println!("[scheduler] initialized with idle task");
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
        if sched.current_time_slice == 0 {
            sched.schedule_next();
        }
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
    let mut sched = SCHEDULER.lock();
    sched.terminate_current(exit_code);
    let _ = sched.schedule_next();
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

/// Yield CPU to next task
pub fn yield_now() {
    let mut sched = SCHEDULER.lock();
    sched.current_time_slice = 0;
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

/// Put current task to sleep for a number of timer ticks.
pub fn sleep_current(ticks: u64) {
    let mut sched = SCHEDULER.lock();
    if let Some(idx) = sched.current_task {
        let wake_tick = crate::drivers::timer::pit::get_ticks().saturating_add(ticks);
        if let Some(ref mut task) = sched.tasks[idx] {
            task.state = TaskState::Sleeping;
            task.wake_tick = Some(wake_tick);
        }
        sched.current_task = None;
        let _ = sched.schedule_next();
    }
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

#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub total_tasks: usize,
    pub ready_tasks: usize,
    pub current_task: Option<alloc::string::String>,
    pub rounds: u64,
}
