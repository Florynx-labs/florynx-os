// =============================================================================
// Florynx Kernel — Round-Robin Scheduler
// =============================================================================
// Implements a simple round-robin scheduler for kernel-level tasks.
// Tasks are cooperative in this initial version: each task runs its entry
// function once per scheduling round.
// =============================================================================

use alloc::collections::VecDeque;
use alloc::string::String;
use lazy_static::lazy_static;
use spin::Mutex;

use super::task::{Task, TaskState};

// The global scheduler instance, protected by a spinlock.
lazy_static! {
    static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

/// Round-robin scheduler managing a queue of tasks.
#[allow(dead_code)]
struct Scheduler {
    /// Ready queue of tasks.
    tasks: VecDeque<Task>,
    /// Total number of scheduling rounds completed.
    rounds: u64,
    /// Whether the scheduler is actively running.
    running: bool,
    /// Tick counter for timer-driven scheduling.
    tick_count: u64,
    /// Number of ticks between scheduling rounds.
    ticks_per_schedule: u64,
}

impl Scheduler {
    const fn new() -> Self {
        Scheduler {
            tasks: VecDeque::new(),
            rounds: 0,
            running: false,
            tick_count: 0,
            ticks_per_schedule: 10, // Schedule every 10 ticks (~100ms at 100Hz)
        }
    }
}

/// Add a new task to the scheduler.
pub fn add_task(task: Task) {
    let mut sched = SCHEDULER.lock();
    crate::serial_println!(
        "[scheduler] added task '{}' (id={})",
        task.name,
        task.id.0
    );
    sched.tasks.push_back(task);
}

/// Add a task from a name and entry function (convenience).
pub fn spawn(name: &str, entry: fn()) {
    add_task(Task::new(name, entry));
}

/// Called by the timer interrupt handler on every tick.
pub fn timer_tick() {
    // Try to lock without blocking — if we can't, skip this tick
    if let Some(mut sched) = SCHEDULER.try_lock() {
        if !sched.running {
            return;
        }
        sched.tick_count += 1;
    }
}

/// Run the scheduler: execute each ready task in round-robin order.
/// This is a cooperative demonstration scheduler.
pub fn run(max_rounds: u64) {
    {
        let mut sched = SCHEDULER.lock();
        sched.running = true;
        let task_count = sched.tasks.len();
        crate::serial_println!(
            "[scheduler] starting with {} tasks, {} rounds",
            task_count,
            max_rounds
        );
        crate::println!(
            "[scheduler] starting with {} tasks, {} rounds",
            task_count,
            max_rounds
        );
    }

    for round in 0..max_rounds {
        let task_count = {
            let sched = SCHEDULER.lock();
            sched.tasks.len()
        };

        for i in 0..task_count {
            let (entry, _name) = {
                let mut sched = SCHEDULER.lock();
                if let Some(task) = sched.tasks.get_mut(i) {
                    task.state = TaskState::Running;
                    task.run_count += 1;
                    (task.entry, task.name.clone())
                } else {
                    continue;
                }
            };

            // Execute the task function (cooperative)
            (entry)();

            // Mark task as ready again
            {
                let mut sched = SCHEDULER.lock();
                if let Some(task) = sched.tasks.get_mut(i) {
                    task.state = TaskState::Ready;
                }
            }
        }

        {
            let mut sched = SCHEDULER.lock();
            sched.rounds = round + 1;
        }
    }

    {
        let mut sched = SCHEDULER.lock();
        sched.running = false;
        crate::serial_println!(
            "[scheduler] completed {} rounds",
            sched.rounds
        );
        crate::println!(
            "[scheduler] completed {} rounds",
            sched.rounds
        );
    }
}

/// Get the number of tasks in the scheduler.
pub fn task_count() -> usize {
    SCHEDULER.lock().tasks.len()
}

/// Get the names of all tasks.
pub fn task_names() -> alloc::vec::Vec<String> {
    let sched = SCHEDULER.lock();
    sched.tasks.iter().map(|t| t.name.clone()).collect()
}
