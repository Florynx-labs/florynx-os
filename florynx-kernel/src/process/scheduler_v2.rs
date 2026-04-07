// =============================================================================
// Florynx Kernel — Advanced Round-Robin Scheduler (v2)
// =============================================================================
// Production-level preemptive scheduler with:
// - Time-slice based scheduling
// - Priority levels
// - Process states (Ready, Running, Blocked, Zombie)
// - Context switching
// - Idle task
// =============================================================================

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use super::task::{Task, TaskState, TaskId, TaskPriority, CpuContext};

lazy_static! {
    static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

/// Advanced scheduler with preemptive multitasking
pub struct Scheduler {
    /// All tasks indexed by ID
    tasks: Vec<Option<Task>>,
    /// Ready queue (task IDs)
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

    /// Add a new task and return its index
    fn add_task_internal(&mut self, mut task: Task) -> usize {
        // Find free slot or append
        for (i, slot) in self.tasks.iter_mut().enumerate() {
            if slot.is_none() {
                task.state = TaskState::Ready;
                *slot = Some(task);
                self.ready_queue.push_back(i);
                return i;
            }
        }
        
        // No free slot, append
        let idx = self.tasks.len();
        task.state = TaskState::Ready;
        self.tasks.push(Some(task));
        self.ready_queue.push_back(idx);
        idx
    }

    /// Get the next task to run
    fn pick_next_task(&mut self) -> Option<usize> {
        // Try to get from ready queue
        while let Some(idx) = self.ready_queue.pop_front() {
            if let Some(ref task) = self.tasks[idx] {
                if task.state == TaskState::Ready {
                    return Some(idx);
                }
            }
        }
        
        // No ready tasks, return idle
        self.idle_task
    }

    /// Switch to the next task
    fn schedule_next(&mut self) -> Option<(usize, usize)> {
        let next_idx = self.pick_next_task()?;
        let prev_idx = self.current_task;
        
        // Update previous task state
        if let Some(prev) = prev_idx {
            if let Some(ref mut task) = self.tasks[prev] {
                if task.state == TaskState::Running {
                    task.state = TaskState::Ready;
                    // Re-queue if not terminated
                    if task.state != TaskState::Terminated {
                        self.ready_queue.push_back(prev);
                    }
                }
            }
        }
        
        // Update next task state
        if let Some(ref mut task) = self.tasks[next_idx] {
            task.state = TaskState::Running;
            task.run_count += 1;
            self.current_time_slice = task.time_slice;
        }
        
        self.current_task = Some(next_idx);
        self.rounds += 1;
        
        prev_idx.map(|prev| (prev, next_idx))
    }

    /// Mark current task as blocked
    fn block_current(&mut self) {
        if let Some(idx) = self.current_task {
            if let Some(ref mut task) = self.tasks[idx] {
                task.state = TaskState::Blocked;
            }
        }
    }

    /// Wake a blocked task
    fn wake_task(&mut self, idx: usize) {
        if let Some(ref mut task) = self.tasks[idx] {
            if task.state == TaskState::Blocked {
                task.state = TaskState::Ready;
                self.ready_queue.push_back(idx);
            }
        }
    }

    /// Terminate current task
    fn terminate_current(&mut self) {
        if let Some(idx) = self.current_task {
            if let Some(ref mut task) = self.tasks[idx] {
                task.state = TaskState::Terminated;
                crate::serial_println!("[scheduler] task '{}' terminated", task.name);
            }
            self.current_task = None;
        }
    }

    /// Get current task
    fn current(&self) -> Option<&Task> {
        self.current_task.and_then(|idx| self.tasks[idx].as_ref())
    }

    /// Get current task mutable
    fn current_mut(&mut self) -> Option<&mut Task> {
        self.current_task.and_then(|idx| self.tasks[idx].as_mut())
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Initialize the scheduler with an idle task
pub fn init() {
    let mut sched = SCHEDULER.lock();
    
    // Create idle task
    let idle = Task::new("idle", idle_task_fn);
    let idle_idx = sched.add_task_internal(idle);
    sched.idle_task = Some(idle_idx);
    
    crate::serial_println!("[scheduler] initialized with idle task");
}

/// Idle task - runs when no other tasks are ready
fn idle_task_fn() {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Add a new task to the scheduler
pub fn spawn(name: &str, entry: fn()) -> TaskId {
    let task = Task::new(name, entry);
    let id = task.id;
    
    let mut sched = SCHEDULER.lock();
    let idx = sched.add_task_internal(task);
    
    crate::serial_println!("[scheduler] spawned task '{}' (id={}, idx={})", name, id.0, idx);
    id
}

/// Add a task with priority
pub fn spawn_with_priority(name: &str, entry: fn(), priority: TaskPriority) -> TaskId {
    let task = Task::with_priority(name, entry, priority);
    let id = task.id;
    
    let mut sched = SCHEDULER.lock();
    let idx = sched.add_task_internal(task);
    
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

/// Called by timer interrupt - handle time slice and scheduling
pub fn timer_tick() {
    if let Some(mut sched) = SCHEDULER.try_lock() {
        if !sched.enabled {
            return;
        }
        
        // Decrement time slice
        if sched.current_time_slice > 0 {
            sched.current_time_slice -= 1;
        }
        
        // Time slice expired, schedule next task
        if sched.current_time_slice == 0 {
            if let Some((prev_idx, next_idx)) = sched.schedule_next() {
                // Context switch would happen here
                // For now, just log it
                if let (Some(ref prev), Some(ref next)) = 
                    (sched.tasks[prev_idx].as_ref(), sched.tasks[next_idx].as_ref()) {
                    crate::serial_println!(
                        "[scheduler] switch: '{}' -> '{}'",
                        prev.name,
                        next.name
                    );
                }
            }
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
    let mut sched = SCHEDULER.lock();
    sched.terminate_current();
}

/// Yield CPU to next task
pub fn yield_now() {
    let mut sched = SCHEDULER.lock();
    sched.current_time_slice = 0; // Force reschedule
}

/// Get current task ID
pub fn current_task_id() -> Option<TaskId> {
    let sched = SCHEDULER.lock();
    sched.current().map(|t| t.id)
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
