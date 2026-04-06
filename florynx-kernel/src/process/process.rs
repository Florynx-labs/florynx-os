// =============================================================================
// Florynx Kernel — Process Structure
// =============================================================================
// Represents a process — a container for one or more tasks with its own
// address space and resource accounting.
// =============================================================================

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::PhysAddr;
use crate::security::isolation::IsolationDomain;
use crate::security::capability::CapabilityTable;

use super::task::TaskId;

/// Unique process identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessId(pub u64);

static NEXT_PID: AtomicU64 = AtomicU64::new(1);

impl ProcessId {
    pub fn new() -> Self {
        ProcessId(NEXT_PID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Process state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Created,
    Running,
    Sleeping,
    Zombie,
}

/// A process — owns an address space and a set of tasks.
pub struct Process {
    pub pid: ProcessId,
    pub name: String,
    pub state: ProcessState,
    /// Task IDs belonging to this process.
    pub tasks: Vec<TaskId>,
    /// Parent process ID (0 = kernel).
    pub parent: ProcessId,
    /// Isolation domain for this process.
    pub isolation: IsolationDomain,
    /// Capabilities granted to this process.
    pub capabilities: CapabilityTable,
    /// Physical address of the level-4 page table for this process.
    pub page_table: Option<PhysAddr>,
}

impl Process {
    pub fn new(name: &str, parent: ProcessId) -> Self {
        Process {
            pid: ProcessId::new(),
            name: String::from(name),
            state: ProcessState::Created,
            tasks: Vec::new(),
            parent,
            isolation: IsolationDomain::user(0), // Default to basic user isolation
            capabilities: CapabilityTable::new(),
            page_table: None,
        }
    }

    pub fn add_task(&mut self, task_id: TaskId) {
        self.tasks.push(task_id);
    }
}

/// Simple process table.
pub struct ProcessTable {
    pub processes: Vec<Process>,
}

impl ProcessTable {
    pub const fn new() -> Self {
        ProcessTable {
            processes: Vec::new(),
        }
    }

    pub fn add(&mut self, process: Process) {
        self.processes.push(process);
    }

    pub fn find(&self, pid: ProcessId) -> Option<&Process> {
        self.processes.iter().find(|p| p.pid == pid)
    }

    pub fn find_mut(&mut self, pid: ProcessId) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }
}
